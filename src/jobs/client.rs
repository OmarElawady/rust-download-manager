use crate::err::ManagerError;
use crate::err::ManagerErrorKind::ChannelError;
use crate::jobs::types::{Delete, Get, JobMessage, List, StateUpdate, Update};
use crate::types::{JobInfo, State};

pub struct StateClient {
    ch: async_channel::Sender<JobMessage>,
}

impl StateClient {
    pub fn new(ch: async_channel::Sender<JobMessage>) -> Self {
        StateClient { ch }
    }
    pub async fn get(&self, name: &str) -> Result<JobInfo, ManagerError> {
        let (s, r) = async_channel::unbounded();
        self.ch
            .send(JobMessage::Get(Get {
                name: name.into(),
                response_channel: s,
            }))
            .await?;
        if let Ok(resp) = r.recv().await {
            return match resp {
                JobMessage::Error(e) => Err(e),
                JobMessage::GetResponse(r) => Ok(r),
                _ => Err(ManagerError {
                    kind: ChannelError,
                    msg: format!("expected a state from the state daemon, got {:?}", resp),
                }),
            };
        }
        Err(ManagerError {
            kind: ChannelError,
            msg: "couldn't get the response from the state daemon".into(),
        })
    }
    pub async fn update(&self, job_state: JobInfo) -> Result<(), ManagerError> {
        let (s, r) = async_channel::unbounded();
        self.ch
            .send(JobMessage::Update(Update {
                job_state,
                response_channel: s,
            }))
            .await?;
        if let Ok(resp) = r.recv().await {
            return match resp {
                JobMessage::Error(e) => Err(e),
                JobMessage::Ack => Ok(()),
                _ => Err(ManagerError {
                    kind: ChannelError,
                    msg: format!("expected an ack from the state daemon, got {:?}", resp),
                }),
            };
        }
        Err(ManagerError {
            kind: ChannelError,
            msg: "couldn't get the response from the state daemon".into(),
        })
    }
    pub async fn delete(&self, name: &str) -> Result<(), ManagerError> {
        let (s, r) = async_channel::unbounded();
        self.ch
            .send(JobMessage::Delete(Delete {
                name: name.into(),
                response_channel: s,
            }))
            .await?;
        if let Ok(resp) = r.recv().await {
            return match resp {
                JobMessage::Error(e) => Err(e),
                JobMessage::Ack => Ok(()),
                _ => Err(ManagerError {
                    kind: ChannelError,
                    msg: format!("expected an ack from the state daemon, got {:?}", resp),
                }),
            };
        }
        Err(ManagerError {
            kind: ChannelError,
            msg: "couldn't get the response from the state daemon".into(),
        })
    }
    pub async fn update_job_state(&self, name: &str, state: State) -> Result<(), ManagerError> {
        let (s, r) = async_channel::unbounded();
        self.ch
            .send(JobMessage::UpdateState(StateUpdate {
                name: name.into(),
                state,
                response_channel: s,
            }))
            .await?;
        if let Ok(resp) = r.recv().await {
            return match resp {
                JobMessage::Error(e) => Err(e),
                JobMessage::Ack => Ok(()),
                _ => Err(ManagerError {
                    kind: ChannelError,
                    msg: format!("expected an ack from the state daemon, got {:?}", resp),
                }),
            };
        }
        Err(ManagerError {
            kind: ChannelError,
            msg: "couldn't get the response from the state daemon".into(),
        })
    }
    pub async fn list(&self) -> Result<Vec<JobInfo>, ManagerError> {
        let (s, r) = async_channel::unbounded();
        self.ch
            .send(JobMessage::List(List {
                response_channel: s,
            }))
            .await?;
        if let Ok(resp) = r.recv().await {
            return match resp {
                JobMessage::Error(e) => Err(e),
                JobMessage::ListResponse(r) => Ok(r),
                _ => Err(ManagerError {
                    kind: ChannelError,
                    msg: format!(
                        "expected a list of states from the state daemon, got {:?}",
                        resp
                    ),
                }),
            };
        }
        Err(ManagerError {
            kind: ChannelError,
            msg: "couldn't get the response from the state daemon".into(),
        })
    }
}
