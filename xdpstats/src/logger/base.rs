use serde::{Deserialize, Serialize};

use crate::logger::level::LogLevel;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct LoggerBase {
    pub log_level: LogLevel,
}

pub type Logger = LoggerBase;

impl LoggerBase {
    pub fn new(log_level: LogLevel) -> Self {
        Self { log_level }
    }
}
