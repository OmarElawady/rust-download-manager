use crate::err::ManagerError;
use crate::types::JobState;
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
    pub cancel_channel: watch::Receiver<bool>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddCommand {
    pub url: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CancelCommand {
    pub name: String,
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
