pub struct DownloadInfo {
    pub name: String,
    pub url: String,
    pub remaining: Option<u64>,
    pub downloaded: Option<u64>,
    pub status: Status,
}

pub enum Status {
    Failed,
    Done,
    Pending,
    Active,
}
