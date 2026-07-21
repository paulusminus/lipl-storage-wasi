use std::string::FromUtf8Error;

use crate::bindings::exports::pm::lipl_core::types::Error;
use crate::bindings::wasi::filesystem::types::ErrorCode;

pub trait ErrInto<T> {
    fn err_into(self) -> Result<T, Error>;
}

impl<T, E: Into<Error>> ErrInto<T> for Result<T, E> {
    fn err_into(self) -> Result<T, Error> {
        self.map_err(Into::into)
    }
}

impl From<ErrorCode> for Error {
    fn from(error_code: ErrorCode) -> Self {
        Self::Io(error_code.to_string())
    }
}

impl From<FromUtf8Error> for Error {
    fn from(error: FromUtf8Error) -> Self {
        Self::Utf8(error.to_string())
    }
}

impl From<toml::de::Error> for Error {
    fn from(error: toml::de::Error) -> Self {
        Self::Toml(error.to_string())
    }
}
