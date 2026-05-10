use crate::logger::{base::LoggerBase, level::LogLevel};

impl LoggerBase {
    #[inline]
    pub fn log_msg(&self, req_level: LogLevel, msg: &str) {
        // Make sure we have the required log level.
        if req_level < self.log_level {
            return;
        }

        // Construct line.
        let line = format!("[{}] {}", req_level, msg);

        // Print basic log line to console
        println!("{}", line);
    }
}
