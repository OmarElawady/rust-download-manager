use crate::err::{ManagerError, ManagerErrorKind};
use crate::types::{JobInfo, State};
use rusqlite::Connection;
use std::str;
pub struct Database {
    conn: Connection,
}
impl rusqlite::types::FromSql for State {
    fn column_result(
        v: rusqlite::types::ValueRef<'_>,
    ) -> std::result::Result<Self, rusqlite::types::FromSqlError> {
        match v {
            rusqlite::types::ValueRef::Text(v) => {
                match str::from_utf8(v)
                    .map_err(|e| rusqlite::types::FromSqlError::Other(Box::new(e)))?
                {
                    "Active" => Ok(Self::Active),
                    "Pending" => Ok(Self::Pending),
                    "Failed" => Ok(Self::Failed),
                    "Cancelled" => Ok(Self::Cancelled),
                    "Done" => Ok(Self::Done),
                    _ => Ok(Self::Unknown),
                }
            }
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}
impl Database {
    pub fn new(path: &str) -> Result<Self, ManagerError> {
        let conn = Connection::open(path)?;
        conn.execute(
            "create table if not exists jobs (
                 name text primary key,
                 url text not null,
                 path text not null,
                 downloaded integer,
                 total integer,
                 state text,
                 msg text
             )",
            [],
        )?;

        Ok(Database { conn })
    }

    pub fn update_state(&self, state: JobInfo) -> Result<(), ManagerError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO jobs (name, url, path, downloaded, total, state, msg)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            [
                state.name,
                state.url,
                state.path,
                state.downloaded.to_string(),
                state.total.to_string(),
                state.state.to_string(),
                state.msg,
            ],
        )?;
        Ok(())
    }
    pub fn update_job_state(&self, name: &str, state: State) -> Result<(), ManagerError> {
        self.conn.execute(
            "UPDATE jobs SET state = ?2 WHERE name = ?1",
            [name, &state.to_string()],
        )?;
        Ok(())
    }

    pub fn delete_job(&self, name: &str) -> Result<(), ManagerError> {
        self.conn
            .execute("DELETE FROM jobs WHERE name=?1", [name])?;
        Ok(())
    }

    pub fn get_job(&self, name: &str) -> Result<JobInfo, ManagerError> {
        let mut stmt = self.conn.prepare(
            "SELECT name, url, path, downloaded, total, state, msg FROM jobs WHERE name = ?1",
        )?;

        let mut jobs = stmt.query_map([name], |row| {
            Ok(JobInfo {
                name: row.get(0)?,
                url: row.get(1)?,
                path: row.get(2)?,
                downloaded: row.get(3)?,
                total: row.get(4)?,
                state: row.get(5)?,
                msg: row.get(6)?,
            })
        })?;
        if let Some(job) = jobs.next() {
            return Ok(job?);
        }
        Err(ManagerError {
            kind: ManagerErrorKind::DownloadJobNotFound,
            msg: format!("{} not found", name),
        })
    }
    pub fn list_jobs(&self) -> Result<Vec<JobInfo>, ManagerError> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, url, path, downloaded, total, state, msg FROM jobs")?;

        let jobs = stmt.query_map([], |row| {
            Ok(JobInfo {
                name: row.get(0)?,
                url: row.get(1)?,
                path: row.get(2)?,
                downloaded: row.get(3)?,
                total: row.get(4)?,
                state: row.get(5)?,
                msg: row.get(6)?,
            })
        })?;
        let mut vs = Vec::new();
        for job in jobs {
            vs.push(job?);
        }
        Ok(vs)
    }
}
