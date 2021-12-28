use crate::err::ManagerError;
use crate::err::ManagerErrorKind::ChannelError;
use crate::state::types::{StateGetMessage, StateListMessage, StateMessage, StateUpdateMessage};
use crate::types::JobState;

pub struct StateClient {
    ch: async_channel::Sender<StateMessage>,
}

impl StateClient {
    pub fn new(ch: async_channel::Sender<StateMessage>) -> Self {
        StateClient { ch }
    }
    pub async fn get(&self, name: &str) -> Result<JobState, ManagerError> {
        let (s, r) = async_channel::unbounded();
        self.ch
            .send(StateMessage::Get(StateGetMessage {
                name: name.into(),
                response_channel: s,
            }))
            .await?;
        while let Ok(resp) = r.recv().await {
            return match resp {
                StateMessage::Error(e) => Err(e),
                StateMessage::GetResponse(r) => Ok(r),
                _ => Err(ManagerError {
                    kind: ChannelError,
                    msg: format!("expected a state from the state daemon, got {:?}", resp).into(),
                }),
            };
        }
        Err(ManagerError {
            kind: ChannelError,
            msg: "couldn't get the response from the state daemon".into(),
        })
    }
    pub async fn update(&self, job_state: JobState) -> Result<(), ManagerError> {
        let (s, r) = async_channel::unbounded();
        self.ch
            .send(StateMessage::Update(StateUpdateMessage {
                job_state,
                response_channel: s,
            }))
            .await?;
        while let Ok(resp) = r.recv().await {
            return match resp {
                StateMessage::Error(e) => Err(e),
                StateMessage::Ack => Ok(()),
                _ => Err(ManagerError {
                    kind: ChannelError,
                    msg: format!("expected an ack from the state daemon, got {:?}", resp).into(),
                }),
            };
        }
        Err(ManagerError {
            kind: ChannelError,
            msg: "couldn't get the response from the state daemon".into(),
        })
    }
    pub async fn list(&self) -> Result<Vec<JobState>, ManagerError> {
        let (s, r) = async_channel::unbounded();
        self.ch
            .send(StateMessage::List(StateListMessage {
                response_channel: s,
            }))
            .await?;
        while let Ok(resp) = r.recv().await {
            return match resp {
                StateMessage::Error(e) => Err(e),
                StateMessage::ListResponse(r) => Ok(r),
                _ => Err(ManagerError {
                    kind: ChannelError,
                    msg: format!(
                        "expected a list of states from the state daemon, got {:?}",
                        resp
                    )
                    .into(),
                }),
            };
        }
        Err(ManagerError {
            kind: ChannelError,
            msg: "couldn't get the response from the state daemon".into(),
        })
    }
}
