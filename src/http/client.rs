use super::types::{Add, Cancel, Error};
use crate::err::ManagerError;
use crate::err::ManagerErrorKind::{DecodingError, HTTPError};
use crate::manager::types::{AckCommand, InfoResponse, ListResponse};
use reqwest;
use url::Url;

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
                msg: e.to_string(),
            }),
            _ => {
                let e = res.json::<Error>().await.map_err(|e| ManagerError {
                    kind: DecodingError,
                    msg: e.to_string(),
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
                msg: e.to_string(),
            }),
            _ => {
                let e = res.json::<Error>().await.map_err(|e| ManagerError {
                    kind: DecodingError,
                    msg: e.to_string(),
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
    pub async fn cancel(
        &self,
        name: &str,
        forget: bool,
        delete: bool,
    ) -> Result<AckCommand, ManagerError> {
        let message = Cancel { forget, delete };
        let url = self.base.join(name)?;

        let res = self.cl.delete(url.as_str()).json(&message).send().await?;
        match res.status() {
            reqwest::StatusCode::OK => Ok(AckCommand),
            _ => {
                let e = res.json::<Error>().await.map_err(|e| ManagerError {
                    kind: DecodingError,
                    msg: e.to_string(),
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
    pub async fn add(&self, url: &str, name: Option<&str>) -> Result<AckCommand, ManagerError> {
        let message = Add {
            url: url.into(),
            name: name.map(|s| s.into()),
        };

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
                    msg: e.to_string(),
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
