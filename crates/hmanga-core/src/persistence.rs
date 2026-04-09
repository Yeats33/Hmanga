use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::models::{DownloadTask, Session};

pub const DOWNLOAD_HISTORY_VERSION: u32 = 1;
pub const READING_PROGRESS_VERSION: u32 = 1;
pub const SESSION_STORE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DownloadHistory {
    pub version: u32,
    pub tasks: Vec<DownloadTask>,
}

impl Default for DownloadHistory {
    fn default() -> Self {
        Self {
            version: DOWNLOAD_HISTORY_VERSION,
            tasks: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionStore {
    pub version: u32,
    pub sessions: HashMap<String, Session>,
}

impl Default for SessionStore {
    fn default() -> Self {
        Self {
            version: SESSION_STORE_VERSION,
            sessions: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadingProgressEntry {
    pub page: u32,
    pub updated_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadingProgressStore {
    pub version: u32,
    pub entries: HashMap<String, ReadingProgressEntry>,
}

impl Default for ReadingProgressStore {
    fn default() -> Self {
        Self {
            version: READING_PROGRESS_VERSION,
            entries: HashMap::new(),
        }
    }
}
