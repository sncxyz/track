use std::fs;

use anyhow::{anyhow, bail, Result};
use bincode::{deserialize, serialize};
use chrono::serde::{ts_seconds, ts_seconds_option};
use serde::{Deserialize, Serialize};

use crate::track::DateTime;

#[derive(Serialize, Deserialize)]
pub struct Data {
    pub current: Option<ActivityInfo>,
    pub all: Vec<ActivityInfo>,
}

impl Data {
    pub fn read() -> Result<Self> {
        Ok(if let Ok(encoded) = fs::read(dir()?.join("data")) {
            deserialize(&encoded)?
        } else {
            Self {
                current: None,
                all: Vec::new(),
            }
        })
    }

    pub fn write(&self) -> Result<()> {
        if !dir()?.exists() {
            fs::create_dir(dir()?)?;
        }
        fs::write(dir()?.join("data"), serialize(self)?)?;
        Ok(())
    }

    pub fn delete(&mut self, i: usize) -> Result<()> {
        let removed = self.all.remove(i);
        if let Some(current) = &self.current {
            if current.id == removed.id {
                self.current = None;
            }
        }
        fs::remove_file(dir()?.join(removed.id.to_string()))?;
        self.write()
    }

    pub fn read_current(&self) -> Result<(Activity, &str)> {
        if let Some(info) = &self.current {
            Ok((
                deserialize(&fs::read(dir()?.join(info.id.to_string()))?)?,
                &info.name,
            ))
        } else {
            bail!("error: No activity currently selected")
        }
    }

    pub fn write_current(&self, activity: &Activity) -> Result<()> {
        activity.write(self.current.as_ref().unwrap().id)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ActivityInfo {
    pub name: String,
    pub id: u32,
}

impl ActivityInfo {
    pub fn new(name: String, id: u32) -> Self {
        Self { name, id }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Activity {
    #[serde(with = "ts_seconds_option")]
    pub ongoing: Option<DateTime>,
    pub sessions: Vec<Session>,
}

impl Activity {
    pub fn new() -> Self {
        Self {
            ongoing: None,
            sessions: Vec::new(),
        }
    }

    pub fn write(&self, id: u32) -> Result<()> {
        fs::write(dir()?.join(id.to_string()), serialize(self)?)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct Session {
    #[serde(with = "ts_seconds")]
    pub start: DateTime,
    #[serde(with = "ts_seconds")]
    pub end: DateTime,
    pub notes: String,
}

impl Session {
    pub fn new(start: DateTime, end: DateTime, notes: String) -> Self {
        Self { start, end, notes }
    }
}

fn dir() -> Result<std::path::PathBuf> {
    Ok(dirs::data_local_dir()
        .ok_or_else(|| anyhow!("error: Failed to find user data directory"))?
        .join("track"))
}
