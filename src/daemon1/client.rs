use crate::daemon1::types::CancelCommand;
use super::stream::ManagerStream;
use crate::daemon1::types::{
    AckCommand, AddCommand, InfoCommand, InfoResponse, ListCommand, ListResponse, Message,
};
use crate::err::ManagerError;
use crate::err::ManagerErrorKind::ChannelError;
use crate::err::ManagerErrorKind::InvalidMessage;
use async_channel::Sender;

pub struct DaemonClient {
    pub ch: Sender<ManagerStream>,
}

impl DaemonClient {
    pub async fn list(&self) -> Result<ListResponse, ManagerError> {
        let (job_sender, job_receiver) = async_channel::unbounded();
        self.ch
            .send(ManagerStream::new(
                Message::List(ListCommand {}),
                job_sender,
            ))
            .await?;
        while let Ok(msg) = job_receiver.recv().await {
            return match msg {
                Message::ListResponse(r) => Ok(r),
                Message::Error(e) => Err(e),
                _ => Err(ManagerError {
                    kind: InvalidMessage,
                    msg: format!("expected a list response from the daemon got {:?}", msg).into(),
                }),
            };
        }
        Err(ManagerError {
            kind: ChannelError,
            msg: "couldn't get the response from the daemon".into(),
        })
    }
    pub async fn info(&self, name: &str) -> Result<InfoResponse, ManagerError> {
        let (job_sender, job_receiver) = async_channel::unbounded();
        self.ch
            .send(ManagerStream::new(
                Message::Info(InfoCommand { name: name.into() }),
                job_sender,
            ))
            .await?;
        while let Ok(msg) = job_receiver.recv().await {
            return match msg {
                Message::InfoResponse(r) => Ok(r),
                Message::Error(e) => Err(e),
                _ => Err(ManagerError {
                    kind: InvalidMessage,
                    msg: format!("expected an info response from the daemon got {:?}", msg).into(),
                }),
            };
        }
        Err(ManagerError {
            kind: ChannelError,
            msg: "couldn't get the response from the daemon".into(),
        })
    }
    pub async fn cancel(&self, name: &str) -> Result<AckCommand, ManagerError> {
        let (job_sender, job_receiver) = async_channel::unbounded();
        self.ch
            .send(ManagerStream::new(
                Message::Cancel(CancelCommand { name: name.into() }),
                job_sender,
            ))
            .await?;
        while let Ok(msg) = job_receiver.recv().await {
            return match msg {
                Message::Ack(_) => Ok(AckCommand),
                Message::Error(e) => Err(e),
                _ => Err(ManagerError {
                    kind: InvalidMessage,
                    msg: format!("expected an info response from the daemon got {:?}", msg).into(),
                }),
            };
        }
        Err(ManagerError {
            kind: ChannelError,
            msg: "couldn't get the response from the daemon".into(),
        })
    }
    pub async fn add(&self, url: &str) -> Result<AckCommand, ManagerError> {
        let (job_sender, job_receiver) = async_channel::unbounded();
        self.ch
            .send(ManagerStream::new(
                Message::Add(AddCommand { url: url.into() }),
                job_sender,
            ))
            .await?;
        while let Ok(msg) = job_receiver.recv().await {
            return match msg {
                Message::Ack(r) => Ok(r),
                Message::Error(e) => Err(e),
                _ => Err(ManagerError {
                    kind: InvalidMessage,
                    msg: format!("expected an ack from the daemon got {:?}", msg).into(),
                }),
            };
        }
        Err(ManagerError {
            kind: ChannelError,
            msg: "couldn't get the response from the daemon".into(),
        })
    }
}
