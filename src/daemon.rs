use crate::err::ManagerErrorKind::ChannelError;
use super::api;
use super::api::Message;
use super::err::ManagerError;
use crate::api::InfoResponse;
use crate::api::ListResponse;
use crate::db::Database;
use crate::http::ManagerStream;
use crate::AckCommand;
use crate::ErrorCommand;
use crate::HTTPListener;
use async_channel;
use reqwest;
use std::fmt;
use std::fmt::Display;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use tokio;
use tokio::time;
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
    Unknown,
}
impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
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
    pub msg: String,
}

pub struct DownloadWorker {
    job_receiver: async_channel::Receiver<DownloadJob>,
    state_client: StateClient,
}

impl DownloadWorker {
    fn new(
        job_receiver: async_channel::Receiver<DownloadJob>,
        state_sender: async_channel::Sender<StateMessage>,
    ) -> Self {
        DownloadWorker {
            job_receiver,
            state_client: StateClient{ ch: state_sender },
        }
    }

    async fn work(self) {
        while let Ok(msg) = self.job_receiver.recv().await {
            self.download(&msg).await
        }
    }
    async fn update_state(&self, state: JobState) {
        let res = self
        .state_client
        .update(state).await;
        if let Err(e) = res {
            println!("failed to update state {}", e)
        }
    }
    async fn download(&self, job: &DownloadJob) {
        let req = reqwest::get(job.url.clone()).await;
        if let Err(e) = req {
            // ignore error
            self.update_state(JobState{
                name: job.name.clone(),
                url: job.url.clone(),
                path: job.file_path.to_str().unwrap().into(), // is this unwrap safe?
                downloaded: 0,
                total: 0,
                state: State::Failed,
                msg: e.to_string(),
            }).await;
            return;
        }
        let mut state = JobState {
            name: job.name.clone(),
            url: job.url.clone(),
            path: job.file_path.to_str().unwrap().into(), // is this unwrap safe?
            downloaded: 0,
            total: 0,
            state: State::Active,
            msg: "".into(),
        };
        self.update_state(state.clone()).await;
        let mut req = req.unwrap();
        let headers = req.headers();
        if let Some(len_str) = headers.get("Content-Length") {
            let len = len_str.to_str().unwrap_or("0").parse().unwrap_or(0);
            state.total = len;
            self.update_state(state.clone()).await;
        }
        let file = File::create(job.file_path.clone());
        if let Err(e) = file {
            state.msg = format!("failed to create file: {}", e.to_string());
            state.state = State::Failed;
            self.update_state(state.clone()).await;
            return;
        }
        let mut file = file.unwrap();
        loop {
            let chunk = req.chunk().await;
            // TODO: there must be a way to chain this
            match chunk {
                Ok(chunk) => match chunk {
                    Some(chunk) => match file.write_all(&chunk) {
                        Ok(_) => {
                            state.downloaded += chunk.len() as u64;
                            self.update_state(state.clone()).await;
                        }
                        Err(e) => {
                            state.msg =
                                format!("failed to write downloaded chunk: {}", e.to_string());
                            state.state = State::Failed;
                            self.update_state(state.clone()).await;
                            return;
                        }
                    },
                    None => {
                        state.state = State::Done;
                        self.update_state(state.clone()).await;
                        return;
                    }
                },
                Err(e) => {
                    state.msg = format!("failed to download chunk: {}", e.to_string());
                    state.state = State::Failed;
                    self.update_state(state.clone()).await;
                    return;
                }
            }
        }
    }
}

pub struct ManagerDaemon {
    server: HTTPListener,
    job_sender: async_channel::Sender<DownloadJob>,
    state_client: StateClient,
}

impl ManagerDaemon {
    // TODO: refactor into a config struct
    pub fn new(workers: u32, listener: HTTPListener) -> Result<Self, ManagerError> {
        let (job_sender, job_receiver) = async_channel::unbounded();
        let (state_sender, state_receiver) = async_channel::unbounded();
        let db = Database::new("/tmp/test.db")?;
        for _ in 0..workers {
            tokio::spawn(DownloadWorker::new(job_receiver.clone(), state_sender.clone()).work());
        }
        tokio::spawn(StateDaemon::new(state_receiver, db).work());
        Ok(ManagerDaemon {
            server: listener,
            job_sender,
            state_client: StateClient{ch: state_sender},
        })
    }
    pub async fn serve(self) -> Result<(), ManagerError> {
        // a single loop handling all the connections
        loop {
            let stream = self.server.next().await;
            if let Err(e) = stream {
                println!("couldn't accept connection {}", e);
                time::sleep(time::Duration::from_secs(1)).await;
                continue;
            }
            let mut stream = stream.unwrap();
            let cmd = match self.handle(&mut stream).await {
                Err(e) => Message::Error(api::ErrorCommand { msg: e.to_string() }),
                Ok(msg) => msg,
            };
            if let Err(e) = stream.write(&cmd).await {
                println!("error sending response {:?}: {}", cmd, e);
            }
            if let Err(e) = stream.shutdown().await {
                // TODO: better logging
                println!("error shutting down client socket {}", e);
            }
        }
    }
    async fn handle(&self, api: &mut ManagerStream) -> Result<Message, ManagerError> {
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
                Ok(Message::Error(ErrorCommand {
                    msg: format!("unexpected command {:?}", cmd).into(),
                }))
            }
        }
    }
    async fn add(&self, url: &str) -> Result<Message, ManagerError> {
        print!("url is: {}", url);
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
            url: url.to_string(),
        };
        self.state_client
            .update(JobState {
                name: name.into(),
                url: url.into(),
                path: file_path.to_str().unwrap().into(), // TODO: unsafe wrap?
                downloaded: 0,
                total: 0,
                state: State::Pending,
                msg: "".into(),
            })
            .await?;
        self.job_sender.send(job).await?;
        Ok(Message::Ack(AckCommand {}))
    }
    async fn list(&self) -> Result<Message, ManagerError> {
        Ok(Message::ListResponse(ListResponse::from(
            self.state_client.list().await?,
        )))
    }
    async fn info(&self, name: &str) -> Result<Message, ManagerError> {
        Ok(Message::InfoResponse(InfoResponse::from(
            &self.state_client.get(name).await?,
        )))
    }
}
#[derive(Debug)]
pub struct StateUpdateMessage {
    job_state: JobState,
    response_channel: async_channel::Sender<StateMessage>,
}
#[derive(Debug)]
pub struct StateGetMessage {
    name: String,
    response_channel: async_channel::Sender<StateMessage>,
}
#[derive(Debug)]
pub struct StateListMessage {
    response_channel: async_channel::Sender<StateMessage>,
}

pub struct StateClient {
    ch: async_channel::Sender<StateMessage>,
}
impl StateClient {
    pub async fn get(&self, name: &str) -> Result<JobState, ManagerError> {
        let (s, r) = async_channel::unbounded();
        self.ch.send(StateMessage::Get(StateGetMessage{
            name: name.into(),
            response_channel: s
        })).await?;
        while let Ok(resp) = r.recv().await {
            return match resp {
                StateMessage::Error(e) => Err(e),
                StateMessage::GetResponse(r) => Ok(r),
                _ => Err(ManagerError {
                    kind: ChannelError,
                    msg: format!("expected a state from the state daemon, got {:?}", resp).into(),
                })
            };
        }
        Err(ManagerError {
            kind: ChannelError,
            msg: "couldn't get the response from the state daemon".into(),
        })
    }
    pub async fn update(&self, job_state: JobState) -> Result<(), ManagerError> {
        let (s, r) = async_channel::unbounded();
        self.ch.send(StateMessage::Update(StateUpdateMessage{
            job_state,
            response_channel: s
        })).await?;
        while let Ok(resp) = r.recv().await {
            return match resp {
                StateMessage::Error(e) => Err(e),
                StateMessage::Ack => Ok(()),
                _ => Err(ManagerError {
                    kind: ChannelError,
                    msg: format!("expected an ack from the state daemon, got {:?}", resp).into(),
                })
            };
        }
        Err(ManagerError {
            kind: ChannelError,
            msg: "couldn't get the response from the state daemon".into(),
        })
    }
    pub async fn list(&self) -> Result<Vec<JobState>, ManagerError> {
        let (s, r) = async_channel::unbounded();
        self.ch.send(StateMessage::List(StateListMessage{
            response_channel: s
        })).await?;
        while let Ok(resp) = r.recv().await {

            return match resp {
                StateMessage::Error(e) => Err(e),
                StateMessage::ListResponse(r) => Ok(r),
                _ => Err(ManagerError {
                    kind: ChannelError,
                    msg: format!("expected a list of states from the state daemon, got {:?}", resp).into(),
                })
            };
        }
        Err(ManagerError {
            kind: ChannelError,
            msg: "couldn't get the response from the state daemon".into(),
        })
    }
}
#[derive(Debug)]
pub enum StateMessage {
    Update(StateUpdateMessage),
    Get(StateGetMessage),
    List(StateListMessage),
    GetResponse(JobState),
    ListResponse(Vec<JobState>),
    Ack,
    Error(ManagerError)
}
pub struct StateDaemon {
    state_receiver: async_channel::Receiver<StateMessage>,
    db: Database,
}

impl StateDaemon {
    fn new(state_receiver: async_channel::Receiver<StateMessage>, db: Database) -> Self {
        return StateDaemon { state_receiver, db };
    }
    async fn work(self) {
        while let Ok(state) = self.state_receiver.recv().await {
            match state {
                StateMessage::Update(msg) => {
                    let res = self.db.update_state(msg.job_state);
                    let _ = match res {
                        Err(e) => msg.response_channel.send(StateMessage::Error(e)).await,
                        Ok(v) => msg.response_channel.send(StateMessage::Ack).await
                    };
                },
                StateMessage::Get(msg) => {
                    let res = self.db.get_state(&msg.name);
                    let _ = match res {
                        Err(e) => msg.response_channel.send(StateMessage::Error(e)).await,
                        Ok(v) => msg.response_channel.send(StateMessage::GetResponse(v)).await
                    };
                },
                StateMessage::List(msg) => {
                    let res = self.db.list_states();
                    let _ = match res {
                        Err(e) => msg.response_channel.send(StateMessage::Error(e)).await,
                        Ok(v) => msg.response_channel.send(StateMessage::ListResponse(v)).await
                    };
                },
                _ => {
                    println!("state daemon got an unexpected message {:?}", state)
                }
            }
        }
        println!("state daemon exited, oh noooo")
    }
}
