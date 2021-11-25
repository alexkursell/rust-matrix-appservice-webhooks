use anyhow::{anyhow, Context, Result};
use matrix_sdk::ruma::RoomId;
use matrix_sdk::ruma::{ServerName, UserId};
use matrix_sdk::SyncSettings;
use sha2::{Digest, Sha256};
use std::{convert::TryFrom, sync::Arc};

use crate::store::Store;
use crate::webhook_request::WebhookRequest;
use crate::{bot, config::Config};
use log::*;
use matrix_sdk_appservice::AppService;
use warp::{Rejection, Reply};

#[derive(Debug, Clone)]
pub struct RequestContext {
  pub config: Arc<Config>,
  pub appservice: AppService,
  pub store: Arc<Store>,
}

pub async fn handler(
  webhook_id: String,
  body: WebhookRequest,
  context: RequestContext,
) -> Result<Box<dyn Reply>, Rejection> {
  let res = handler_inner(
    &webhook_id,
    body,
    context.config,
    context.appservice,
    context.store,
  )
  .await;
  Ok(match res {
    Ok(_) => Box::new(warp::reply::json(&serde_json::json!({"success": true}))),
    Err(e) => {
      error!(
        "Error responding to webhook request with id {}: {}",
        &webhook_id,
        e.to_string()
      );
      Box::new(warp::reply::with_status(
        warp::reply::json(&serde_json::json!({"success": false, "message": e.to_string()})),
        http::status::StatusCode::INTERNAL_SERVER_ERROR,
      ))
    }
  })
}

async fn handler_inner(
  webhook_id: &str,
  body: WebhookRequest,
  config: Arc<Config>,
  appservice: AppService,
  store: Arc<Store>,
) -> Result<()> {
  debug!("Received webhook for id {}", webhook_id);
  let hook = match store.get_webhook_by_id(webhook_id).await? {
    Some(hook) => hook,
    None => return Err(anyhow::anyhow!("Could not find webhook")),
  };

  let room_id = RoomId::try_from(hook.room_id)?;

  let mut hasher = Sha256::new();
  hasher.update(&hook.id);
  let id_hash = hex::encode(&hasher.finalize()[0..16]);
  let bot_localpart = format!("{}__{}", &config.webhook_bot.localpart, &id_hash);

  let client = bot::register_bot(
    &bot_localpart,
    &body.display_name,
    &body.avatar_url,
    appservice.clone(),
  )
  .await?;

  // May be over-cautious
  client.sync_once(SyncSettings::default()).await?;

  // Have the bot invite the webhook to the room only if it's not already joined
  if client.get_joined_room(&room_id).is_none() {
    let bot_client = appservice
      .virtual_user_client(&config.webhook_bot.localpart)
      .await?;
    let room = bot_client
      .get_joined_room(&room_id)
      .map_or(Err(anyhow!("Couldn't get joined room from bot")), Ok)?;

    room
      .invite_user_by_id(&UserId::parse_with_server_name(
        bot_localpart.as_str(),
        <&ServerName>::try_from(config.homeserver.domain.as_str())?,
      )?)
      .await
      .context("Failed to have bot invite the webhook")?;

    client.join_room_by_id(&room_id).await?;
  }

  client
    .room_send(&room_id, body.create_message(), None)
    .await?;

  Ok(())
}
