use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs::File;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
  pub homeserver: Homeserver,
  pub webhook_bot: Bot,
  pub provisioning: Provisioning,
  pub web: Web,
  pub logging: Logging,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Homeserver {
  pub url: String,
  pub domain: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bot {
  pub localpart: String,
  pub appearance: Appearance,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Appearance {
  pub display_name: String,
  pub avatar_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provisioning {
  secret: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Web {
  pub hook_url_base: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Logging {
  file: String,
  console: bool,
  console_level: String,
  file_level: String,
  write_files: bool,
  rotate: LoggingRotate,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingRotate {
  size: u64,
  count: u64,
}

pub fn from_file(path: &str) -> Result<Config> {
  let file = File::open(path).with_context(|| format!("Failed to open config file at {}", path))?;
  serde_yaml::from_reader(file).context("Failed to parse config file")
}
