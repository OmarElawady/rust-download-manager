use crate::daemon::JobState;
use crate::err::ManagerError;
use serde;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;
use std::fmt::Display;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net;
#[derive(Serialize, Deserialize, Debug)]
pub enum CommandKind {
    Add,
    List,
    Info,
    Ack,
    Error,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddCommand {
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorCommand {
    pub msg: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InfoCommand {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InfoResponse {
    pub name: String,
    pub url: String,
    pub path: String,
    pub downloaded: u64,
    pub total: u64,
    pub state: String, // should it be State?
    pub msg: String,
}
impl Display for InfoResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "name: {}\n", self.name)?;
        write!(f, "url: {}\n", self.url)?;
        write!(f, "path: {}\n", self.path)?;
        write!(f, "downloaded: {}\n", self.downloaded)?;
        if self.total != 0 {
            write!(f, "total: {}\n", self.total)?;
        }
        write!(f, "state: {}\n", self.state)?;
        if self.msg != "" {
            write!(f, "msg: {}\n", self.msg)?;
        }
        Ok(())
    }
}
impl From<&JobState> for InfoResponse {
    fn from(s: &JobState) -> Self {
        InfoResponse {
            name: s.name.clone(),
            url: s.url.clone(),
            path: s.path.clone(),
            downloaded: s.downloaded,
            total: s.total,
            state: s.state.to_string(),
            msg: s.msg.clone(),
        }
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ListResponse {
    pub entries: Vec<InfoResponse>,
}
impl Display for ListResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        for e in self.entries.iter() {
            write!(f, "- {}:", e.name)?;
            write!(f, " [{}]", e.state)?;
            if e.total != 0 {
                write!(f, " [{}/{}]", e.downloaded, e.total)?;
            } else {
                write!(f, " [{}]", e.downloaded)?;
            }
            write!(f, "\n")?;
        }
        Ok(())
    }
}
impl From<Vec<JobState>> for ListResponse {
    fn from(v: Vec<JobState>) -> Self {
        let mut entries = Vec::new();
        for e in v.iter() {
            entries.push(InfoResponse::from(e));
        }
        Self { entries }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListCommand;

#[derive(Serialize, Deserialize, Debug)]
pub struct AckCommand;

// TODO: this should be an enum
#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    Add(AddCommand),
    List(ListCommand),
    ListResponse(ListResponse),
    Info(InfoCommand),
    InfoResponse(InfoResponse),
    Ack(AckCommand),
    Error(ErrorCommand),
}

pub struct ManagerApi {
    endpoint: net::TcpStream,
}

impl From<net::TcpStream> for ManagerApi {
    fn from(endpoint: net::TcpStream) -> Self {
        ManagerApi { endpoint }
    }
}
// rename to ManagerStream
impl ManagerApi {
    pub async fn new_client(addr: &str) -> Result<Self, ManagerError> {
        Ok(Self::from(net::TcpStream::connect(addr).await?))
    }

    pub async fn write(&mut self, cmd: &Message) -> Result<(), ManagerError> {
        let x = bincode::serialize(&cmd).unwrap();
        self.endpoint.write_u64(x.len() as u64).await?;
        self.endpoint.write_all(&x).await?;
        Ok(())
    }

    pub async fn read(&mut self) -> Result<Message, ManagerError> {
        let len = self.endpoint.read_u64().await?;
        let mut buf = vec![0; len as usize];
        self.endpoint.read_exact(&mut buf).await?;
        let cmd = bincode::deserialize(&buf)?;
        Ok(cmd)
    }

    pub async fn shutdown(&mut self) -> Result<(), ManagerError> {
        self.endpoint.shutdown().await?;
        Ok(())
    }
}
