
pub use ::anyhow::*;

// a trait if you don't care about providing context to errors
pub trait Anyhow<T, E> {
    fn anyhow(self) -> Result<T, Error>;
}

use std::{error::Error as StdError, convert::Infallible};

impl<T, E> Anyhow<T, E> for Result<T, E>
    where E: StdError + Send + Sync + 'static
{
    fn anyhow(self) -> Result<T, Error> {
        self.map_err(|err| anyhow!(err))
    }
}

impl<T> Anyhow<T, Infallible> for Option<T> {
    fn anyhow(self) -> Result<T, Error> {
        self.context("None")
    }
}