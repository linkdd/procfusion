use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;


#[derive(Debug, Clone, Deserialize)]
pub struct Config {
  pub processes: HashMap<String, Process>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Process {
  pub command: String,
  pub directory: Option<PathBuf>,
  pub shell: Option<String>,
}

impl Config {
  pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
    let content = std::fs::read_to_string(path)?;
    let config = toml::from_str(&content)?;
    Ok(config)
  }
}
