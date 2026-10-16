use rusqlite::Connection;
use std::sync::{atomic::AtomicBool, Mutex};

use crate::settings::Settings;

#[derive(Debug, Clone, PartialEq)]
pub enum ClipFingerprint {
    Text(String),
    Image(String),
    Files(Vec<String>),
}

pub struct AppState {
    pub db: Mutex<Connection>,
    pub settings: Mutex<Settings>,
    pub paused: AtomicBool,
    pub baseline: Mutex<Option<ClipFingerprint>>,
}

impl AppState {
    pub fn new(db: Connection, settings: Settings) -> Self {
        Self {
            db: Mutex::new(db),
            settings: Mutex::new(settings),
            paused: AtomicBool::new(false),
            baseline: Mutex::new(None),
        }
    }
}
