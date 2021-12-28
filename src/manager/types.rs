use crate::err::ManagerError;
use crate::types::JobInfo;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;
use std::fmt::Display;
use std::path::PathBuf;
use tokio::sync::watch;

pub struct DownloadJob {
    pub name: String,
    pub url: String,
    pub file_path: PathBuf,
    pub cancel_channel: watch::Receiver<CancelInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddCommand {
    pub url: String,
    pub name: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CancelCommand {
    pub name: String,
    pub forget: bool,
    pub delete: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ErrorCommand {
    pub msg: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InfoCommand {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
        writeln!(f, "name: {}", self.name)?;
        writeln!(f, "url: {}", self.url)?;
        writeln!(f, "path: {}", self.path)?;
        writeln!(f, "downloaded: {}", self.downloaded)?;
        if self.total != 0 {
            writeln!(f, "total: {}", self.total)?;
        }
        writeln!(f, "state: {}", self.state)?;
        if !self.msg.is_empty() {
            writeln!(f, "msg: {}", self.msg)?;
        }
        Ok(())
    }
}
impl From<&JobInfo> for InfoResponse {
    fn from(s: &JobInfo) -> Self {
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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListResponse(Vec<InfoResponse>);

impl Display for ListResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        for e in self.0.iter() {
            write!(f, "- {}:", e.name)?;
            write!(f, " [{}]", e.state)?;
            if e.total != 0 {
                write!(f, " [{}/{}]", e.downloaded, e.total)?;
            } else {
                write!(f, " [{}]", e.downloaded)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
impl From<Vec<JobInfo>> for ListResponse {
    fn from(v: Vec<JobInfo>) -> Self {
        let mut entries = Vec::new();
        for e in v.iter() {
            entries.push(InfoResponse::from(e));
        }
        ListResponse(entries)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListCommand;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AckCommand;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    Add(AddCommand),
    List(ListCommand),
    ListResponse(ListResponse),
    Info(InfoCommand),
    InfoResponse(InfoResponse),
    Cancel(CancelCommand),
    Ack(AckCommand),
    Error(ManagerError),
}

#[derive(Debug)]
pub struct CancelInfo {
    pub cancel: bool,
    pub delete: bool,
}

impl Display for CancelInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "cancel: {}, delete: {}", self.cancel, self.delete)
    }
}
