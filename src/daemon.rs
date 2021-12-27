use crate::ErrorCommand;
use crate::api::ListResponse;
use crate::api::InfoResponse;
use std::path::Path;
use crate::AckCommand;
use std::sync::Arc;
use crate::db::Database;
use tokio::sync::Mutex;
use std::fmt::Display;
use super::api::Message;
use super::api;
use super::api::ManagerApi;
use super::err::ManagerError;
use tokio::net;
use tokio::time;
use std::path::PathBuf;
use tokio;
use async_channel;
use reqwest;
use std::fs::File;
use std::io::prelude::*;
use std::fmt;
use url::Url;
pub struct DownloadJob {
    name: String,
    url: String,
    file_path: PathBuf,
}

#[derive(Clone, Debug)]
pub enum State {
    Active,
    Pending,
    Failed,
    Done,
    Unknown
}
impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", 
            match *self {
                State::Active => "Active",
                State::Pending => "Pending",
                State::Failed => "Failed",
                State::Done => "Done",
                State::Unknown => "Unknown",
            }
        )
    }
}
#[derive(Clone, Debug)]
pub struct JobState {
    pub name: String,
    pub url: String,
    pub path: String,
    pub downloaded: u64,
    pub total: u64,
    pub state: State,
    pub msg: String
}

pub struct DownloadWorker {
    job_receiver: async_channel::Receiver<DownloadJob>,
    state_sender: async_channel::Sender<JobState>
}

impl DownloadWorker {
    fn new(job_receiver: async_channel::Receiver<DownloadJob>, state_sender: async_channel::Sender<JobState>) -> Self {
        DownloadWorker{job_receiver, state_sender}
    }

    async fn work(self) {
        while let Ok(msg) = self.job_receiver.recv().await {
            self.download(&msg).await
        }
    }

    async fn download(&self, job: &DownloadJob) {
        let req = reqwest::get(job.url.clone()).await;
        if let Err(e) = req {
            // ignore error
            let _ = self.state_sender.send(JobState{
                name: job.name.clone(),
                url: job.url.clone(),
                path: job.file_path.to_str().unwrap().into(), // is this unwrap safe?    
                downloaded: 0,
                total: 0,
                state: State::Failed,
                msg: e.to_string()
            }).await;
            return
        }
        let mut state = JobState{
            name: job.name.clone(),
            url: job.url.clone(),
            path: job.file_path.to_str().unwrap().into(), // is this unwrap safe?
            downloaded: 0,
            total: 0,
            state: State::Active,
            msg: "".into()
        };
        let _ = self.state_sender.send(state.clone()).await;
        let mut req = req.unwrap();
        let headers = req.headers();
        if let Some(len_str) = headers.get("Content-Length") {
            let len = len_str.to_str().unwrap_or("0").parse().unwrap_or(0);
            state.total = len;
            let _ = self.state_sender.send(state.clone()).await;
        }
        let file = File::create(job.file_path.clone());
        if let Err(e) = file {
            state.msg = format!("failed to create file: {}", e.to_string());
            state.state = State::Failed;
            let _ = self.state_sender.send(state.clone()).await;
            return
        }
        let mut file = file.unwrap();
        loop {
            let chunk = req.chunk().await;
            // TODO: there must be a way to chain this
            match chunk {
                Ok(chunk) => {
                    match chunk {
                        Some(chunk) => {
                            match file.write_all(&chunk) {
                                Ok(_) => {
                                    state.downloaded += chunk.len() as u64;
                                    let _ = self.state_sender.send(state.clone()).await;
                                },
                                Err(e) => {
                                    state.msg = format!("failed to write downloaded chunk: {}", e.to_string());
                                    state.state = State::Failed;
                                    let _ = self.state_sender.send(state.clone()).await;
                                    return
                                }
                            }
                        },
                        None => {
                            state.state = State::Done;
                            let _ = self.state_sender.send(state.clone()).await;
                            return
                        }
                    }
                },
                Err(e) => {
                    state.msg = format!("failed to download chunk: {}", e.to_string());
                    state.state = State::Failed;
                    let _ = self.state_sender.send(state.clone()).await;
                    return
                }
            }
        }
    }
}

pub struct ManagerDaemon {
    server: net::TcpListener,
    job_sender: async_channel::Sender<DownloadJob>,
    state_sender: async_channel::Sender<JobState>,
    db: Arc<Mutex<Database>>
}

impl ManagerDaemon {
    // TODO: refactor into a config struct
    pub fn new(addr: &str, workers: u32) -> Result<Self, ManagerError> {
        let addr = addr.parse()?;
        let socket = net::TcpSocket::new_v4()?;
        socket.bind(addr)?;

        let listener = socket.listen(1024)?;
        let (job_sender, job_receiver) = async_channel::unbounded();
        let (state_sender, state_receiver) = async_channel::unbounded();
        let db = Arc::new(Mutex::new(Database::new("/tmp/test.db")?));
        for _ in 0..workers {
            tokio::spawn(DownloadWorker::new(job_receiver.clone(), state_sender.clone()).work());
        }
        tokio::spawn(StateDaemon::new(state_receiver, db.clone()).work());
        Ok(ManagerDaemon { server: listener, job_sender, state_sender, db})
    }
    pub async fn serve(&self) -> Result<(), ManagerError> {
        // a single loop handling all the connections
        loop {
            let socket = self.server.accept().await;
            if let Err(e) = socket {
                println!("couldn't accept connection {}", e);
                time::sleep(time::Duration::from_secs(1)).await;
                continue;
            }
            let mut api = ManagerApi::from(socket.unwrap().0);
            let cmd = match self.handle(&mut api).await {
                Err(e) => Message::Error(api::ErrorCommand{msg: e.to_string()}),
                Ok(msg) => msg,
            };
            if let Err(e) = api.write(&cmd).await {
                println!("error sending response {:?}: {}", cmd, e);
            }
            if let Err(e) = api.shutdown().await {
                // TODO: better logging
                println!("error shutting down client socket {}", e);
            }
        }
    }
    async fn handle(&self, api: &mut ManagerApi) -> Result<Message, ManagerError> {
        let cmd = api.read().await?;
        match cmd {
            Message::Add(c) => {
                println!("adding {}", c.url);
                Ok(self.add(&c.url).await?)
            }
            Message::Info(c) => {
                println!("querying {}", c.name);
                Ok(self.info(&c.name).await?)
            }
            Message::List(_) => {
                println!("listing");
                Ok(self.list().await?)
            }
            _ => {
                println!("thanks for the acknowledgment, don't expect it though");
                Ok(Message::Error(ErrorCommand{msg: format!("unexpected command {:?}", cmd).into()}))
            }
        }
    }
    async fn add(&self, url: &str) -> Result<Message, ManagerError> {
        let u = Url::parse(url)?;
        let segments = u.path_segments();
        let mut name = "unnamed";
        if let Some(segments) = segments {
            name = segments.last().unwrap_or("unnamed");
        }
        let file_path = Path::new("/tmp/Downloads").join(name);
        let job = DownloadJob {
            name: name.into(),
            // TODO: make configurable
            file_path: file_path.clone(),
            url: url.to_string()
        };
        self.state_sender.send(JobState{
            name: name.into(),
            url: url.into(),
            path: file_path.to_str().unwrap().into(), // TODO: unsafe wrap?
            downloaded: 0,
            total: 0,
            state: State::Pending,
            msg: "".into()
        }).await?;
        self.job_sender.send(job).await?;
        Ok(Message::Ack(AckCommand{}))
    }
    async fn list(&self) -> Result<Message, ManagerError> {
        Ok(Message::ListResponse(ListResponse::from(self.db.lock().await.list_states()?)))
    }
    async fn info(&self, name: &str) -> Result<Message, ManagerError> {
        Ok(Message::InfoResponse(InfoResponse::from(&self.db.lock().await.get_state(name)?)))
    }
}


pub struct StateDaemon {
    state_receiver: async_channel::Receiver<JobState>,
    db: Arc<Mutex<Database>>
}

impl StateDaemon {
    fn new(state_receiver: async_channel::Receiver<JobState>, db: Arc<Mutex<Database>>) -> Self {
        return StateDaemon{state_receiver, db}
    }

    async fn work(self) {
        while let Ok(state) = self.state_receiver.recv().await {
            if let Err(e) = self.db.lock().await.update_state(state) {
                println!("error updating state in the db {}", e)
            }
        }
        println!("state daemon exited, oh noooo")
    }
}