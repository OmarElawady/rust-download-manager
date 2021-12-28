use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ManagerErrorKind {
    IO,
    InvalidAddress,
    InvalidMessage,
    DecodingError,
    DatabaseError,
    ChannelError,
    HTTPError,
    DownloadJobNotFound,
    ParseIntError
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ManagerError {
    pub kind: ManagerErrorKind,
    pub msg: String,
}
impl fmt::Display for ManagerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.msg)
    }
}
impl fmt::Display for ManagerErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ManagerErrorKind::DecodingError => "decoding error".to_string(),
                ManagerErrorKind::InvalidAddress => "invalid address error".to_string(),
                ManagerErrorKind::IO => "io error".to_string(),
                ManagerErrorKind::DatabaseError => "db error".to_string(),
                ManagerErrorKind::ChannelError => "channel error".to_string(),
                ManagerErrorKind::InvalidMessage => "invalid message".to_string(),
                ManagerErrorKind::HTTPError => "http error".to_string(),
                ManagerErrorKind::DownloadJobNotFound => "download job not found".to_string(),
                ManagerErrorKind::ParseIntError => "error parsing integer".to_string(),
            }
        )
    }
}
impl From<std::io::Error> for ManagerError {
    fn from(err: std::io::Error) -> Self {
        return ManagerError {
            kind: ManagerErrorKind::IO,
            msg: err.to_string(),
        };
    }
}
impl From<std::net::AddrParseError> for ManagerError {
    fn from(err: std::net::AddrParseError) -> Self {
        return ManagerError {
            kind: ManagerErrorKind::InvalidAddress,
            msg: err.to_string(),
        };
    }
}
impl From<std::string::FromUtf8Error> for ManagerError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        return ManagerError {
            kind: ManagerErrorKind::DecodingError,
            msg: err.to_string(),
        };
    }
}
impl From<std::boxed::Box<bincode::ErrorKind>> for ManagerError {
    fn from(err: std::boxed::Box<bincode::ErrorKind>) -> Self {
        return ManagerError {
            kind: ManagerErrorKind::DecodingError,
            msg: err.to_string(),
        };
    }
}

impl From<rusqlite::Error> for ManagerError {
    fn from(err: rusqlite::Error) -> Self {
        return ManagerError {
            kind: ManagerErrorKind::DatabaseError,
            msg: err.to_string(),
        };
    }
}
impl From<url::ParseError> for ManagerError {
    fn from(err: url::ParseError) -> Self {
        return ManagerError {
            kind: ManagerErrorKind::InvalidAddress,
            msg: err.to_string(),
        };
    }
}
impl From<async_channel::SendError<crate::state::types::StateMessage>> for ManagerError {
    fn from(err: async_channel::SendError<crate::state::types::StateMessage>) -> Self {
        return ManagerError {
            kind: ManagerErrorKind::ChannelError,
            msg: err.to_string(),
        };
    }
}

impl From<async_channel::SendError<crate::daemon1::types::DownloadJob>> for ManagerError {
    fn from(err: async_channel::SendError<crate::daemon1::types::DownloadJob>) -> Self {
        return ManagerError {
            kind: ManagerErrorKind::ChannelError,
            msg: err.to_string(),
        };
    }
}
impl From<async_channel::SendError<crate::daemon1::types::Message>> for ManagerError {
    fn from(err: async_channel::SendError<crate::daemon1::types::Message>) -> Self {
        return ManagerError {
            kind: ManagerErrorKind::ChannelError,
            msg: err.to_string(),
        };
    }
}

impl From<async_channel::SendError<crate::daemon1::stream::ManagerStream>> for ManagerError {
    fn from(err: async_channel::SendError<crate::daemon1::stream::ManagerStream>) -> Self {
        return ManagerError {
            kind: ManagerErrorKind::ChannelError,
            msg: err.to_string(),
        };
    }
}

impl From<tokio::sync::watch::error::SendError<bool>> for ManagerError {
    fn from(err: tokio::sync::watch::error::SendError<bool>) -> Self {
        return ManagerError {
            kind: ManagerErrorKind::ChannelError,
            msg: err.to_string(),
        };
    }
}

impl From<rocket::Error> for ManagerError {
    fn from(err: rocket::Error) -> Self {
        return ManagerError {
            kind: ManagerErrorKind::HTTPError,
            msg: err.to_string(),
        };
    }
}

impl From<reqwest::Error> for ManagerError {
    fn from(err: reqwest::Error) -> Self {
        return ManagerError {
            kind: ManagerErrorKind::HTTPError,
            msg: err.to_string(),
        };
    }
}
impl From<std::num::ParseIntError> for ManagerError {
    fn from(err: std::num::ParseIntError) -> Self {
        return ManagerError {
            kind: ManagerErrorKind::ParseIntError,
            msg: err.to_string(),
        };
    }
}
