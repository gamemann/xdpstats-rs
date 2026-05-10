use crate::{logger::level::LogLevel, watcher::base::LogBuffer};

pub const BACKLOG_DEFAULT_SZ: usize = 200;

#[derive(Clone, Default)]
pub struct LoggerBase {
    pub log_level: LogLevel,
    pub buffer: Option<LogBuffer>,

    pub backlog: usize,
}

pub type Logger = LoggerBase;

impl LoggerBase {
    pub fn new(log_level: LogLevel, buffer: Option<LogBuffer>, backlog: usize) -> Self {
        Self {
            log_level,
            buffer,
            backlog,
        }
    }
}
