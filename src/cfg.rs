use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;


#[derive(Debug, Deserialize)]
pub struct Config {
  pub processes: HashMap<String, Process>,
}

#[derive(Debug, Deserialize)]
pub struct Process {
  pub command: String,
  pub directory: Option<PathBuf>,
}

impl Config {
  pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
    let content = std::fs::read_to_string(path)?;
    let config = toml::from_str(&content)?;
    Ok(config)
  }
}
