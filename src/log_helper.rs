
// log helper
#[macro_export]
macro_rules! log_warn {($msg:expr) => ($crate::log::warn!("{:?}", $msg))}

#[macro_export]
macro_rules! log_err {($msg:expr) => ($crate::log::error!("{:?}", $msg))}