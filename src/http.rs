use crate::daemon1::types::{AckCommand, InfoResponse, ListResponse};
use crate::err::ManagerError;
use crate::err::ManagerErrorKind::DecodingError;
use crate::err::ManagerErrorKind::HTTPError;
use crate::rest::Error;
use reqwest;
use rocket::serde::{Deserialize, Serialize};
use url::Url;

#[derive(Serialize, Deserialize)]
pub struct Add {
    url: String,
}

pub struct HTTPClient {
    base: Url,
    cl: reqwest::Client,
}

impl HTTPClient {
    pub async fn new(url: &str) -> Result<Self, ManagerError> {
        let url = Url::parse(url)?;
        let url = url.join("api/v1/jobs/")?;
        Ok(Self {
            base: url,
            cl: reqwest::Client::new(),
        })
    }
    pub async fn list(&self) -> Result<ListResponse, ManagerError> {
        let res = self.cl.get(self.base.as_str()).send().await?;
        match res.status() {
            reqwest::StatusCode::OK => res.json().await.map_err(|e| ManagerError {
                kind: DecodingError,
                msg: e.to_string().into(),
            }),
            _ => {
                let e = res.json::<Error>().await.map_err(|e| ManagerError {
                    kind: DecodingError,
                    msg: e.to_string().into(),
                });
                match e {
                    Ok(v) => Err(ManagerError {
                        kind: HTTPError,
                        msg: v.to_string(),
                    }),
                    Err(e) => Err(e),
                }
            }
        }
    }
    pub async fn info(&self, name: &str) -> Result<InfoResponse, ManagerError> {
        let url = self.base.join(name)?;

        let res = self.cl.get(url.as_str()).send().await?;
        match res.status() {
            reqwest::StatusCode::OK => res.json().await.map_err(|e| ManagerError {
                kind: DecodingError,
                msg: e.to_string().into(),
            }),
            _ => {
                let e = res.json::<Error>().await.map_err(|e| ManagerError {
                    kind: DecodingError,
                    msg: e.to_string().into(),
                });
                match e {
                    Err(e) => Err(e),
                    Ok(v) => Err(ManagerError {
                        kind: HTTPError,
                        msg: v.to_string(),
                    }),
                }
            }
        }
    }
    pub async fn cancel(&self, name: &str) -> Result<AckCommand, ManagerError> {
        let url = self.base.join(name)?;

        let res = self.cl.delete(url.as_str()).send().await?;
        match res.status() {
            reqwest::StatusCode::OK => Ok(AckCommand),
            _ => {
                let e = res.json::<Error>().await.map_err(|e| ManagerError {
                    kind: DecodingError,
                    msg: e.to_string().into(),
                });
                match e {
                    Err(e) => Err(e),
                    Ok(v) => Err(ManagerError {
                        kind: HTTPError,
                        msg: v.to_string(),
                    }),
                }
            }
        }
    }
    pub async fn add(&self, url: &str) -> Result<AckCommand, ManagerError> {
        let message = Add { url: url.into() };

        let res = self
            .cl
            .post(self.base.as_str())
            .json(&message)
            .send()
            .await?;
        match res.status() {
            reqwest::StatusCode::CREATED => Ok(AckCommand),
            _ => {
                let e = res.json::<Error>().await.map_err(|e| ManagerError {
                    kind: DecodingError,
                    msg: e.to_string().into(),
                });
                match e {
                    Err(e) => Err(e),
                    Ok(v) => Err(ManagerError {
                        kind: HTTPError,
                        msg: v.to_string(),
                    }),
                }
            }
        }
    }
}
