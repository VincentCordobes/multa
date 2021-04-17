use std::io;
use std::num::ParseIntError;

pub type Result<T> = std::result::Result<T, ErrorKind>;

#[derive(Debug)]
pub enum ErrorKind {
    Crossterm(crossterm::ErrorKind),
    Serde(serde_json::Error),
    Io(io::Error),
    InvalidAnswer(ParseIntError),
    Exit,
}

impl From<crossterm::ErrorKind> for ErrorKind {
    fn from(err: crossterm::ErrorKind) -> ErrorKind {
        ErrorKind::Crossterm(err)
    }
}

impl From<serde_json::Error> for ErrorKind {
    fn from(err: serde_json::Error) -> ErrorKind {
        ErrorKind::Serde(err)
    }
}

impl From<io::Error> for ErrorKind {
    fn from(err: io::Error) -> ErrorKind {
        ErrorKind::Io(err)
    }
}
