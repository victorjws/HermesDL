use std::fmt;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Segment {
    pub url: Arc<String>,
    pub start: u64,
    pub end: u64,
}

impl Segment {
    pub fn new(url: Arc<String>, start: u64, end: u64) -> Self {
        Self { url, start, end }
    }

    pub fn get_range_header(&self) -> String {
        format!("bytes={}-{}", self.start, self.end)
    }
}

impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Segment(url:{},start:{},end:{})",
            self.url, self.start, self.end
        )
    }
}
