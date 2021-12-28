use super::types::Message;
use crate::err::ManagerError;
use async_channel::Sender;

pub struct ManagerStream {
    msg: Message,
    response_channel: Sender<Message>,
}

impl ManagerStream {
    pub fn new(msg: Message, response_channel: Sender<Message>) -> Self {
        ManagerStream {
            msg,
            response_channel,
        }
    }
    pub async fn read(&mut self) -> Result<Message, ManagerError> {
        Ok(self.msg.clone())
    }
    pub async fn write(&mut self, cmd: &Message) -> Result<(), ManagerError> {
        self.response_channel.send(cmd.clone()).await?;
        Ok(())
    }
}
