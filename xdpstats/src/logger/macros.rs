#[macro_export]
macro_rules! log {
    ($logger:expr, $level:expr, $($arg:tt)*) => {
        $logger.log_msg($level, &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! trace {
    ($logger:expr, $($arg:tt)*) => {
        $crate::log!($logger, $crate::logger::level::LogLevel::Trace, $($arg)*)
    };
}

#[macro_export]
macro_rules! debug {
    ($logger:expr, $($arg:tt)*) => {
        $crate::log!($logger, $crate::logger::level::LogLevel::Debug, $($arg)*)
    };
}

#[macro_export]
macro_rules! info {
    ($logger:expr, $($arg:tt)*) => {
        $crate::log!($logger, $crate::logger::level::LogLevel::Info, $($arg)*)
    };
}

#[macro_export]
macro_rules! warn {
    ($logger:expr, $($arg:tt)*) => {
        $crate::log!($logger, $crate::logger::level::LogLevel::Warn, $($arg)*)
    };
}

#[macro_export]
macro_rules! error {
    ($logger:expr, $($arg:tt)*) => {
        $crate::log!($logger, $crate::logger::level::LogLevel::Error, $($arg)*)
    };
}

#[macro_export]
macro_rules! fatal {
    ($logger:expr, $($arg:tt)*) => {
        $crate::log!($logger, $crate::logger::level::LogLevel::Fatal, $($arg)*)
    };
}
