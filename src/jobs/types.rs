use crate::err::ManagerError;
use crate::types::{JobInfo, State};

#[derive(Debug)]
pub struct Update {
    pub job_state: JobInfo,
    pub response_channel: async_channel::Sender<JobMessage>,
}
#[derive(Debug)]
pub struct StateUpdate {
    pub name: String,
    pub state: State,
    pub response_channel: async_channel::Sender<JobMessage>,
}
#[derive(Debug)]
pub struct Get {
    pub name: String,
    pub response_channel: async_channel::Sender<JobMessage>,
}
#[derive(Debug)]
pub struct Delete {
    pub name: String,
    pub response_channel: async_channel::Sender<JobMessage>,
}
#[derive(Debug)]
pub struct List {
    pub response_channel: async_channel::Sender<JobMessage>,
}

#[derive(Debug)]
pub enum JobMessage {
    Update(Update),
    Delete(Delete),
    UpdateState(StateUpdate),
    Get(Get),
    List(List),
    GetResponse(JobInfo),
    ListResponse(Vec<JobInfo>),
    Ack,
    Error(ManagerError),
}
