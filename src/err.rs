use std::fmt;

#[derive(Debug)]
pub enum ManagerErrorKind {
    ConnectionError,
    IO,
    InvalidAddress,
    DecodingError,
    DatabaseError,
    ChannelError,
}
#[derive(Debug)]
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
                ManagerErrorKind::ConnectionError => "connection error".to_string(),
                ManagerErrorKind::DatabaseError => "db error".to_string(),
                ManagerErrorKind::ChannelError => "channel error".to_string(),
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
impl From<async_channel::SendError<crate::daemon::JobState>> for ManagerError {
    fn from(err: async_channel::SendError<crate::daemon::JobState>) -> Self {
        return ManagerError {
            kind: ManagerErrorKind::ChannelError,
            msg: err.to_string(),
        };
    }
}

impl From<async_channel::SendError<crate::daemon::DownloadJob>> for ManagerError {
    fn from(err: async_channel::SendError<crate::daemon::DownloadJob>) -> Self {
        return ManagerError {
            kind: ManagerErrorKind::ChannelError,
            msg: err.to_string(),
        };
    }
}
