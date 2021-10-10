use std::{net::IpAddr, str::FromStr, sync::Arc};

use anyhow::{Context, Result};
use clap::Clap;
use log::*;
use matrix_sdk::{
  room::Room,
  ruma::events::{
    room::{member::MemberEventContent, message::MessageEventContent},
    SyncMessageEvent, SyncStateEvent,
  },
  SyncSettings,
};
use matrix_sdk_appservice::{AppService, AppServiceRegistration};
use tokio::sync::oneshot;
use warp::Filter;

mod bot;
mod config;
mod emoji;
mod store;
mod webhook;
mod webhook_request;

#[derive(Debug, Clap)]
#[clap(
  version = "0.1.0",
  author = "Alex Kursell <alex@awk.run>",
  about = "Matrix appservice for slack-like webhooks"
)]
struct Opts {
  #[clap(short = 'p', long)]
  port: u16,

  #[clap(short = 'c', long)]
  config_file: String,

  #[clap(short = 'f', long)]
  registration_file: String,

  #[clap(short = 'd', long)]
  database_path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
  env_logger::init_from_env(env_logger::Env::default().filter_or(
    env_logger::DEFAULT_FILTER_ENV,
    "debug,sled=warn,sqlx=warn,html5ever=warn",
  ));
  let opts: Opts = Opts::parse();

  info!("Reading config files");
  let config = Arc::new(config::from_file(&opts.config_file)?);
  let homeserver_url = config.homeserver.url.as_str();
  let server_name = config.homeserver.domain.as_str();
  let registration = AppServiceRegistration::try_from_yaml_file(&opts.registration_file)?;
  let appservice = AppService::new(homeserver_url, server_name, registration).await?;

  info!("Opening database connection");
  let store = Arc::new(store::Store::connect(&opts.database_path).await?);
  let request_context = webhook::RequestContext {
    config: config.clone(),
    store: store.clone(),
    appservice: appservice.clone(),
  };

  // The handler needs the webhook id from the path, the config object, the appservice object
  // and a database connection (TODO)
  let webhook_filter = warp::path!("api" / "v1" / "matrix" / "hook" / String)
    .and(warp::filters::method::post())
    .and(warp::filters::body::json())
    .and(warp::any().map({
      let request_context = request_context.clone();
      move || request_context.clone()
    }))
    .and_then(webhook::handler);

  info!("Starting appservice");
  // Start the web server
  let (tx, rx) = oneshot::channel();
  let (server_addr, server) = warp::serve(appservice.warp_filter().or(webhook_filter))
    .bind_with_graceful_shutdown((IpAddr::from_str("::0").unwrap(), opts.port), async {
      rx.await.ok();
      info!("Appservice received termination signal. Shutting down webserver");
    });

  tokio::task::spawn(server);
  info!("Server running on {}", server_addr);

  // First, register the @_webhook bot and set hooks for it to respond to invites and !webhook messages
  let client = bot::register_bot(
    &config.webhook_bot.localpart,
    &config.webhook_bot.appearance.display_name,
    &config.webhook_bot.appearance.avatar_url,
    appservice.clone(),
  )
  .await
  .context("Failed to register bot with homeserver")?;

  // Do a full sync to make sure bot knows about all of the rooms it's in
  client
    .sync_once(SyncSettings::new().full_state(true))
    .await?;

  // Handle invites for the webhook bot to rooms
  client
    .register_event_handler({
      let appservice = appservice.clone();
      let config = config.clone();
      move |event: SyncStateEvent<MemberEventContent>, room: Room| {
        bot::handle_room_member(config.clone(), appservice.clone(), room, event)
      }
    })
    .await;

  // Handle !webhook requests
  client
    .register_event_handler({
      let appservice = appservice.clone();
      let config = config.clone();
      let store = store.clone();
      move |event: SyncMessageEvent<MessageEventContent>, room: Room| {
        bot::handle_room_message(
          config.clone(),
          store.clone(),
          appservice.clone(),
          room,
          event,
        )
      }
    })
    .await;

  info!("Waiting for termination signal");
  tokio::signal::ctrl_c().await?;
  info!("Received termination signal");
  let _ = tx.send(());
  Ok(())
}
