use std::{fmt, io};

#[derive(Debug)]
pub enum DownloadError {
    Io(io::Error),
    Network(String),
    Parse(String),
    Logic(String), // This probably should be a panic instead tbh. Logic errors in client code
    // shouldn't be like this
    HashMismatch { expected: String, actual: String },
    Args(String),
}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DownloadError::Io(e) => write!(f, "IO Error: {e}"),
            DownloadError::Network(s) => write!(f, "Network Error: {s}"),
            DownloadError::Parse(s) => write!(f, "Response Parse Error: {s}"),
            DownloadError::Logic(s) => write!(f, "Logic Error: {s}"),
            DownloadError::HashMismatch { expected, actual } => {
                write!(
                    f,
                    "Hash HashMismatch!\n Expected: {expected}\n Actual:  {actual}"
                )
            }
            DownloadError::Args(s) => write!(f, "Argument Error: {s}"),
        }
    }
}

impl std::error::Error for DownloadError {}

impl From<io::Error> for DownloadError {
    fn from(err: io::Error) -> Self {
        DownloadError::Io(err)
    }
}

impl From<std::num::ParseIntError> for DownloadError {
    fn from(value: std::num::ParseIntError) -> Self {
        DownloadError::Parse(format!("Failed to parse integer: {value}"))
    }
}

impl From<std::str::Utf8Error> for DownloadError {
    fn from(value: std::str::Utf8Error) -> Self {
        DownloadError::Parse(format!("Failed to parse UTF-8 string: {value}"))
    }
}
