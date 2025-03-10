use super::progress::{ProgressBar, ProgressManager};
use super::segment::Segment;
use crate::request::{client::Client, response::Response, user_agent::UserAgent};
use anyhow::{anyhow, Result};
use bytes::Bytes;
use futures_core::Stream;
use std::cmp::min;
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tokio_stream::StreamExt;
use url::Url;

#[cfg(feature = "unix")]
use std::os::unix::fs::FileExt;

#[cfg(feature = "windows")]
use std::os::windows::fs::FileExt;

#[derive(Clone, Debug)]
pub struct Downloader {
    client: Client,
    segment_size: u64,
    max_concurrent: usize,
}

impl Downloader {
    pub fn new(
        use_tor: bool,
        user_agent: &UserAgent,
        segment_size: u64,
        max_concurrent: usize,
    ) -> Self {
        Self {
            client: Client::new(use_tor, user_agent).unwrap(),
            segment_size,
            max_concurrent,
        }
    }

    pub async fn download_file(
        &self,
        url: &str,
        headers: Option<&HashMap<String, String>>,
    ) -> Result<()> {
        let head_response = self.client.head(url, headers).await?;
        let content_type = head_response.content_type();
        let filename = self.get_filename(&head_response, url);
        let progress_manager = Arc::new(ProgressManager::new(filename.clone()));

        if url.ends_with(".m3u8")
            || content_type
                .map(|ct| ct.contains("application/vnd.apple.mpegurl"))
                .unwrap_or(false)
        {
            // ts playlist
            let filename = if filename.ends_with(".m3u8") {
                let ts_path = Path::new(&filename).with_extension("ts");
                ts_path.to_string_lossy().to_string()
            } else {
                filename.to_string()
            };
            progress_manager
                .main_progress_bar
                .write()
                .await
                .set_name(filename.clone());

            let get_response = self.client.get(url, headers).await?;
            let base_url = Url::parse(url)?;
            let playlist: Vec<String> = get_response
                .text()
                .await
                .unwrap_or_default()
                .lines()
                .filter_map(|line| {
                    if (!line.starts_with("#")) && line.contains(".") {
                        let ts_url = if let Ok(absolute_url) = Url::parse(line) {
                            Some(absolute_url.to_string())
                        } else {
                            base_url.join(line).ok().map(|u| u.to_string())
                        };
                        if let Some(url) = ts_url {
                            return Some(url);
                        }
                    }
                    None
                })
                .collect();

            let segments = self.get_segments_info(playlist).await?;
            let total_size: u64 = segments.iter().map(|s| s.end - s.start + 1).sum();

            progress_manager
                .main_progress_bar
                .read()
                .await
                .set_length(total_size);

            self.download_parallel(
                segments,
                headers,
                &filename,
                false,
                progress_manager.clone(),
            )
            .await?;
        } else {
            // normal file
            let content_length = head_response.content_length();
            let accept_ranges = head_response.accept_ranges();

            match (content_length, accept_ranges) {
                (Some(content_length), Some(accept_ranges)) if accept_ranges == "bytes" => {
                    progress_manager
                        .main_progress_bar
                        .read()
                        .await
                        .set_length(content_length);

                    let url_arc = Arc::new(url.to_string());
                    let mut segments = Vec::new();

                    for offset in (0..content_length).step_by(self.segment_size as usize) {
                        let end = min(offset + self.segment_size - 1, content_length - 1);
                        let segment = Segment::new(url_arc.clone(), offset, end);
                        segments.push(segment);
                    }
                    self.download_parallel(
                        segments,
                        headers,
                        &filename,
                        true,
                        progress_manager.clone(),
                    )
                    .await?;
                }
                _ => {
                    self.download_full(url, &filename, progress_manager.clone())
                        .await?
                }
            }
        }

        progress_manager.main_progress_bar.read().await.finish();

        Ok(())
    }

    fn get_filename(&self, response: &Response, url: &str) -> String {
        let filename = if let Some(content_disposition) = response.content_disposition() {
            if let Some(filename) = content_disposition.split("filename=").nth(1) {
                return filename.trim_matches('"').to_string();
            } else {
                "downloaded_file.ts".to_string()
            }
        } else if let Ok(parsed_url) = Url::parse(url) {
            if let Some(filename) = Path::new(parsed_url.path()).file_name() {
                return filename.to_string_lossy().to_string();
            } else {
                "downloaded_file.ts".to_string()
            }
        } else {
            "downloaded_file.ts".to_string()
        };

        let mut path = PathBuf::from(&filename);
        let parent = Path::new("../files");

        let mut count = 1;
        while path.exists() {
            let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let extension = path.extension().unwrap_or_default().to_string_lossy();
            let new_filename = if extension.is_empty() {
                format!("{} ({})", file_stem, count)
            } else {
                format!("{} ({}).{}", file_stem, count, extension)
            };

            path = parent.join(&new_filename);
            count += 1;
        }

        path.file_name().unwrap().to_string_lossy().to_string()
    }

    async fn get_segments_info(&self, urls: Vec<String>) -> Result<Vec<Segment>> {
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));
        let mut handles = Vec::new();

        for url in urls.iter().cloned() {
            let client = self.client.clone();
            let permit = semaphore.clone().acquire_owned().await?;

            let handle = tokio::spawn(async move {
                let _permit = permit;
                match client.head(&url, None).await {
                    Ok(response) => response.content_length(),
                    Err(_) => None,
                }
            });
            handles.push(handle);
        }

        let mut segments = Vec::new();
        let mut start = 0u64;

        for (handle, url) in handles.into_iter().zip(urls.iter()) {
            let size = match handle.await {
                Ok(Some(size)) => size,
                Ok(None) => {
                    eprintln!("Fail to get size {}", url);
                    continue;
                }
                Err(e) => {
                    eprintln!("Fail to get size {}, caused {}", url, e);
                    continue;
                }
            };
            let segment = Segment::new(Arc::from(url.clone()), start, start + size - 1);
            segments.push(segment);
            start += size;
        }

        Ok(segments)
    }

    async fn download_parallel(
        &self,
        segments: Vec<Segment>,
        default_headers: Option<&HashMap<String, String>>,
        output_path: &str,
        accept_ranges: bool,
        progress_manager: Arc<ProgressManager>,
    ) -> Result<()> {
        let file = Arc::new(
            OpenOptions::new()
                .create(true)
                .write(true)
                .open("files/".to_owned() + output_path)?,
        );
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));

        let mut handles = vec![];

        for (index, segment) in segments.iter().enumerate() {
            let self_clone = self.clone();
            let client = self.client.clone();
            let segment = segment.clone();
            let total = segments.len();
            let default_headers = default_headers.cloned();
            let main_progress_bar = progress_manager.main_progress_bar.clone();
            let progress_bar = progress_manager.create_new_progress_bar(
                segment.end - segment.start + 1,
                format!("{}/{}", index + 1, total),
            );
            let permit = semaphore.clone().acquire_owned().await?;
            let file = Arc::clone(&file);

            let handle = tokio::spawn(async move {
                let _permit = permit;
                let mut headers = if default_headers.is_some() {
                    default_headers.unwrap().clone()
                } else {
                    HashMap::<String, String>::new()
                };
                if accept_ranges {
                    headers.insert("Range".to_string(), segment.get_range_header());
                }

                self_clone
                    .retryable_get_segment(
                        &client,
                        &file,
                        &segment,
                        &headers,
                        main_progress_bar,
                        progress_bar,
                    )
                    .await
                    .ok()
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.await?;
        }

        Ok(())
    }

    async fn retryable_get_segment(
        &self,
        client: &Client,
        file: &Arc<File>,
        segment: &Segment,
        headers: &HashMap<String, String>,
        global_progress_bar: Arc<RwLock<ProgressBar>>,
        local_progress_bar: ProgressBar,
    ) -> Result<()> {
        let max_retries = 3;
        let mut attempts = 0;

        while attempts < max_retries {
            match self
                .get_segment(
                    client,
                    file,
                    segment,
                    headers,
                    global_progress_bar.clone(),
                    &local_progress_bar,
                )
                .await
            {
                Ok(_) => return Ok(()),
                Err(e) => {
                    eprintln!(
                        "Failed to download segment: {}, Chunk {}-{} retrying {}/{}...",
                        e, &segment.start, &segment.end, attempts, max_retries
                    );
                    attempts += 1;

                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
            }
        }

        Err(anyhow!(
            "Failed to download chunk {}-{} after {} attempts",
            segment.start,
            segment.end,
            max_retries
        ))
    }

    async fn get_segment(
        &self,
        client: &Client,
        file: &Arc<File>,
        segment: &Segment,
        headers: &HashMap<String, String>,
        global_progress_bar: Arc<RwLock<ProgressBar>>,
        local_progress_bar: &ProgressBar,
    ) -> Result<()> {
        match client.get(&segment.url, Some(&headers)).await {
            Ok(response) => {
                self.write_segment(
                    response.bytes_stream(),
                    file,
                    segment.start,
                    global_progress_bar,
                    local_progress_bar,
                )
                .await?;
                Ok(())
            }
            Err(e) => Err(anyhow!(
                "Failed to download segment {}: {}, Chunk {}-{}",
                segment.url,
                e,
                segment.start,
                segment.end,
            )),
        }
    }

    async fn write_segment(
        &self,
        mut stream: impl Stream<Item = Result<Bytes>> + Unpin,
        file: &Arc<File>,
        start: u64,
        global_progress_bar: Arc<RwLock<ProgressBar>>,
        local_progress_bar: &ProgressBar,
    ) -> Result<()> {
        // let mut file = file.lock().await;
        let mut offset = start;
        // file.seek(SeekFrom::Start(offset)).await?;

        while let Some(chunk) = stream.next().await {
            if let Ok(chunk) = chunk {
                file.write_at(&chunk, offset)?;
                let len = chunk.len() as u64;
                local_progress_bar.increase(len);
                global_progress_bar.read().await.increase(len);
                offset += len;
            } else {
                return Err(anyhow!("Failed to download segment {}", offset));
            }
        }

        file.sync_all()?;
        local_progress_bar.finish_and_clear();
        Ok(())
    }

    async fn download_full(
        &self,
        url: &str,
        output_path: &str,
        progress_manager: Arc<ProgressManager>,
    ) -> Result<()> {
        let response = self.client.get(url, None).await?;
        let file = File::create(output_path)?;
        let mut stream = response.bytes_stream();
        let mut offset = 0u64;

        while let Some(chunk) = stream.next().await {
            if let Ok(chunk) = chunk {
                file.write_at(&chunk, offset)?;
                let len = chunk.len() as u64;
                progress_manager
                    .main_progress_bar
                    .read()
                    .await
                    .increase(len);
                offset += len;
            } else {
                return Err(anyhow!("Failed to write file: {}", output_path));
            }
        }

        file.sync_all()?;

        Ok(())
    }
}

impl fmt::Display for Downloader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Downloader({},{},{})",
            self.client, self.segment_size, self.max_concurrent
        )
    }
}
