
use std::string::ToString;

// Results and error Handling
pub type Error = String;

// error constructor, works with all types that implement Display
pub fn error(err: impl ToString) -> Error {
    err.to_string()
}

pub type Res<T> = Result<T, Error>;

pub trait ConvertResult<T, E> {
    fn convert(self) -> Res<T>;
}

impl<T, E: ToString> ConvertResult<T, E> for Result<T, E> {
    fn convert(self) -> Res<T> { self.map_err(error) }
}

// log helper
#[macro_export]
macro_rules! log_warn {($msg:expr) => ($crate::log::warn!("{}", $msg))}
#[macro_export]
macro_rules! log_err {($msg:expr) => ($crate::log::error!("{}", $msg))}

#[macro_export]
macro_rules! log_warn_dbg {($msg:expr) => ($crate::log::warn!("{:?}", $msg))}
#[macro_export]
macro_rules! log_err_dbg {($msg:expr) => ($crate::log::error!("{:?}", $msg))}