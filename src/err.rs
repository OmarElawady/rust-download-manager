use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(PartialEq, Debug, Deserialize, Serialize, Clone)]
pub enum ManagerErrorKind {
    IO,
    InvalidAddress,
    InvalidMessage,
    DecodingError,
    DatabaseError,
    ChannelError,
    HTTPError,
    DownloadJobNotFound,
    DownloadJobNameAlreadyExist,
    ParseIntError,
    ParseBoolError,
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
                ManagerErrorKind::ParseBoolError => "errorr parsing bool".to_string(),
                ManagerErrorKind::DownloadJobNameAlreadyExist =>
                    "download job name already exist".to_string(),
            }
        )
    }
}
impl From<std::io::Error> for ManagerError {
    fn from(err: std::io::Error) -> Self {
        ManagerError {
            kind: ManagerErrorKind::IO,
            msg: err.to_string(),
        }
    }
}
impl From<std::net::AddrParseError> for ManagerError {
    fn from(err: std::net::AddrParseError) -> Self {
        ManagerError {
            kind: ManagerErrorKind::InvalidAddress,
            msg: err.to_string(),
        }
    }
}
impl From<std::string::FromUtf8Error> for ManagerError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        ManagerError {
            kind: ManagerErrorKind::DecodingError,
            msg: err.to_string(),
        }
    }
}

impl From<rusqlite::Error> for ManagerError {
    fn from(err: rusqlite::Error) -> Self {
        ManagerError {
            kind: ManagerErrorKind::DatabaseError,
            msg: err.to_string(),
        }
    }
}
impl From<url::ParseError> for ManagerError {
    fn from(err: url::ParseError) -> Self {
        ManagerError {
            kind: ManagerErrorKind::InvalidAddress,
            msg: err.to_string(),
        }
    }
}
impl From<async_channel::SendError<crate::jobs::types::JobMessage>> for ManagerError {
    fn from(err: async_channel::SendError<crate::jobs::types::JobMessage>) -> Self {
        ManagerError {
            kind: ManagerErrorKind::ChannelError,
            msg: err.to_string(),
        }
    }
}

impl From<async_channel::SendError<crate::manager::types::DownloadJob>> for ManagerError {
    fn from(err: async_channel::SendError<crate::manager::types::DownloadJob>) -> Self {
        ManagerError {
            kind: ManagerErrorKind::ChannelError,
            msg: err.to_string(),
        }
    }
}
impl From<async_channel::SendError<crate::manager::types::Message>> for ManagerError {
    fn from(err: async_channel::SendError<crate::manager::types::Message>) -> Self {
        ManagerError {
            kind: ManagerErrorKind::ChannelError,
            msg: err.to_string(),
        }
    }
}

impl From<async_channel::SendError<crate::manager::stream::ManagerStream>> for ManagerError {
    fn from(err: async_channel::SendError<crate::manager::stream::ManagerStream>) -> Self {
        ManagerError {
            kind: ManagerErrorKind::ChannelError,
            msg: err.to_string(),
        }
    }
}

impl From<tokio::sync::watch::error::SendError<crate::manager::types::CancelInfo>>
    for ManagerError
{
    fn from(err: tokio::sync::watch::error::SendError<crate::manager::types::CancelInfo>) -> Self {
        ManagerError {
            kind: ManagerErrorKind::ChannelError,
            msg: format!("error sending cancelation to the worker {:?}", err),
        }
    }
}

impl From<rocket::Error> for ManagerError {
    fn from(err: rocket::Error) -> Self {
        ManagerError {
            kind: ManagerErrorKind::HTTPError,
            msg: err.to_string(),
        }
    }
}

impl From<reqwest::Error> for ManagerError {
    fn from(err: reqwest::Error) -> Self {
        ManagerError {
            kind: ManagerErrorKind::HTTPError,
            msg: err.to_string(),
        }
    }
}
impl From<std::num::ParseIntError> for ManagerError {
    fn from(err: std::num::ParseIntError) -> Self {
        ManagerError {
            kind: ManagerErrorKind::ParseIntError,
            msg: err.to_string(),
        }
    }
}
impl From<std::str::ParseBoolError> for ManagerError {
    fn from(err: std::str::ParseBoolError) -> Self {
        ManagerError {
            kind: ManagerErrorKind::ParseBoolError,
            msg: err.to_string(),
        }
    }
}
