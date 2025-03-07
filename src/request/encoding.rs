use anyhow::Error;
use async_compression::tokio::bufread::{
    BrotliDecoder, BzDecoder, Deflate64Decoder, GzipDecoder, LzmaDecoder, XzDecoder, ZlibDecoder,
    ZstdDecoder,
};
use std::str::FromStr;
use tokio::io::{AsyncRead, AsyncReadExt, BufReader};

type Decoder = Box<dyn AsyncRead + Unpin + Send>;

pub enum ContentDecoder {
    Brotli,
    Bzip2,
    Deflate,
    Gzip,
    Lzma,
    Xz,
    Zlib,
    Zstd,
    Deflate64,
    Unknown,
}

impl ContentDecoder {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentDecoder::Brotli => "brotli",
            ContentDecoder::Bzip2 => "bzip2",
            ContentDecoder::Deflate => "deflate",
            ContentDecoder::Gzip => "gzip",
            ContentDecoder::Lzma => "lzma",
            ContentDecoder::Xz => "xz",
            ContentDecoder::Zlib => "zlib",
            ContentDecoder::Zstd => "zstd",
            ContentDecoder::Deflate64 => "dflate64",
            _ => "unknown",
        }
    }

    pub async fn decode<R: AsyncRead + Unpin + Send + 'static>(self, reader: R) -> String {
        let buf_reader = BufReader::new(reader);
        let mut decoded_string = String::new();
        let mut decoder = match self {
            ContentDecoder::Brotli => Box::new(BrotliDecoder::new(buf_reader)) as Decoder,
            ContentDecoder::Bzip2 => Box::new(BzDecoder::new(buf_reader)) as Decoder,
            ContentDecoder::Deflate => Box::new(ZlibDecoder::new(buf_reader)) as Decoder,
            ContentDecoder::Gzip => Box::new(GzipDecoder::new(buf_reader)) as Decoder,
            ContentDecoder::Lzma => Box::new(LzmaDecoder::new(buf_reader)) as Decoder,
            ContentDecoder::Xz => Box::new(XzDecoder::new(buf_reader)) as Decoder,
            ContentDecoder::Zlib => Box::new(ZlibDecoder::new(buf_reader)) as Decoder,
            ContentDecoder::Zstd => Box::new(ZstdDecoder::new(buf_reader)) as Decoder,
            ContentDecoder::Deflate64 => Box::new(Deflate64Decoder::new(buf_reader)) as Decoder,
            ContentDecoder::Unknown => {
                let mut plain_reader = buf_reader;
                plain_reader
                    .read_to_string(&mut decoded_string)
                    .await
                    .unwrap();
                return decoded_string;
            }
        };
        decoder.read_to_string(&mut decoded_string).await.unwrap();
        decoded_string
    }
}

impl FromStr for ContentDecoder {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let decoder_type = match s {
            "brotli" => ContentDecoder::Brotli,
            "bzip2" => ContentDecoder::Bzip2,
            "deflate" => ContentDecoder::Deflate,
            "gzip" => ContentDecoder::Gzip,
            "lzma" => ContentDecoder::Lzma,
            "xz" => ContentDecoder::Xz,
            "zlib" => ContentDecoder::Zlib,
            "zstd" => ContentDecoder::Zstd,
            "dflate64" => ContentDecoder::Deflate64,
            _ => ContentDecoder::Unknown,
        };
        Ok(decoder_type)
    }
}

impl PartialEq<str> for ContentDecoder {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}
