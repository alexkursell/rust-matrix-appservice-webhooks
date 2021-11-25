use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs::File;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
  pub homeserver: Homeserver,
  pub webhook_bot: Bot,
  pub web: Web,
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
pub struct Web {
  pub hook_url_base: String,
}

pub fn from_file(path: &str) -> Result<Config> {
  let file = File::open(path).with_context(|| format!("Failed to open config file at {}", path))?;
  serde_yaml::from_reader(file).context("Failed to parse config file")
}
