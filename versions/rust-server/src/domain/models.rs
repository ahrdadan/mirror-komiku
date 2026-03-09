use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterMeta {
    pub source_url: String,
    pub next_url: Option<String>,
    pub generated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub title: String,
    pub image_count: usize,
    #[serde(default)]
    pub total_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct ParsedChapter {
    pub title: String,
    pub image_urls: Vec<Url>,
    pub next_url: Option<Url>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    ChapterInit {
        chapter_hash: String,
        title: String,
        total_images: usize,
        raw_first_three: Vec<String>,
        raw_remaining: Vec<String>,
        next_mirror_path: Option<String>,
    },
    RawChapterInit {
        chapter_hash: String,
        title: String,
        total_images: usize,
        raw_first_three: Vec<String>,
        raw_remaining: Vec<String>,
        next_raw_path: Option<String>,
    },
    ImageAvif {
        chapter_hash: String,
        index: usize,
        url: String,
    },
    ChapterDone {
        chapter_hash: String,
    },
    PrefetchedChapter {
        chapter_hash: String,
        source_url: String,
        title: String,
        image_urls: Vec<String>,
    },
    RawPrefetchedChapter {
        chapter_hash: String,
        source_url: String,
        title: String,
        image_urls: Vec<String>,
        next_raw_path: Option<String>,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Web,
    Worker,
    All,
}
