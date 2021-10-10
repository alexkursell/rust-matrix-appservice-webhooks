use crate::{config, store::Store};
use anyhow::{anyhow, Context};
use matrix_sdk::{
  media::MediaFormat,
  ruma::{
    api::client::r0::room::create_room::RoomPreset,
    events::{room::message::MessageType, AnyMessageEventContent, SyncMessageEvent},
    RoomId, ServerName,
  },
};
use std::{convert::TryFrom, sync::Arc};

use matrix_sdk::ruma::api::client::r0::room::create_room::Request as CreateRoomRequest;

use matrix_sdk_appservice::{
  matrix_sdk::{
    room::Room,
    ruma::{
      events::{
        room::member::{MemberEventContent, MembershipState},
        room::message::MessageEventContent,
        SyncStateEvent,
      },
      UserId,
    },
    Client,
  },
  AppService, Result,
};

use log::*;

pub async fn handle_room_member(
  config: Arc<config::Config>,
  appservice: AppService,
  room: Room,
  event: SyncStateEvent<MemberEventContent>,
) -> Result<()> {
  let room_id = room.room_id().to_string();
  let event_copy = event.clone();
  let result = handle_room_member_inner(config, appservice, room, event).await;
  if let Err(err) = result {
    error!(
      "Error handling membership event for room {}, {:?}: {}",
      room_id,
      event_copy,
      err.to_string()
    );
  }
  Ok(())
}

pub async fn handle_room_message(
  config: Arc<config::Config>,
  store: Arc<Store>,
  appservice: AppService,
  room: Room,
  event: SyncMessageEvent<MessageEventContent>,
) -> Result<()> {
  let room_id = room.room_id().to_string();
  let result = handle_room_message_inner(config, store, appservice, room, event).await;
  if let Err(err) = result {
    error!(
      "Error handling message for room {}: {}",
      room_id,
      err.to_string()
    );
  }

  Ok(())
}

pub async fn register_bot(
  localpart: &str,
  display_name: &str,
  avatar_url: &str,
  appservice: AppService,
) -> anyhow::Result<Client> {
  info!("Registering the webhook bot with the homeserver");
  appservice.register_virtual_user(localpart).await?;
  let client = appservice.virtual_user_client(localpart).await?;

  client
    .set_display_name(Some(display_name))
    .await
    .context("Failed to set bot display name")?;

  // Allow updating the avatar to fail
  match download_avatar(avatar_url).await {
    Ok((avatar_mime, avatar_bytes)) => {
      let mut slice = avatar_bytes.as_slice();
      let old_avatar_bytes = client.avatar(MediaFormat::File).await?;
      if old_avatar_bytes.is_none() || (old_avatar_bytes.unwrap().as_slice() != slice) {
        client
          .upload_avatar(&avatar_mime, &mut slice)
          .await
          .context("Failed to upload fetched avatar to homeserver")?;
      }
    }
    Err(e) => {
      warn!(
        "Failed to download bot avatar from {}: {}",
        avatar_url,
        e.to_string()
      );
    }
  };

  Ok(client)
}

async fn handle_room_message_inner(
  config: Arc<config::Config>,
  store: Arc<Store>,
  appservice: AppService,
  room: Room,
  event: SyncMessageEvent<MessageEventContent>,
) -> anyhow::Result<()> {
  let text_msg = match event.content.msgtype {
    MessageType::Text(t) => t,
    _ => return Ok(()),
  };

  if !text_msg.body.starts_with("!webhook") {
    return Ok(());
  }

  info!(
    "Received !webhook message in room {}. Creating webhook",
    room.room_id().to_string()
  );

  // Register webhook for room
  let client = appservice
    .virtual_user_client(&config.webhook_bot.localpart)
    .await?;

  let admin_room_id = get_or_create_admin_room(&client, &event.sender)
    .await
    .context("Failed to get or create admin room")?;
  let admin_room = match client.get_joined_room(&admin_room_id) {
    Some(room) => room,
    None => Err(anyhow!("Failed to get the room that we should be inside"))?,
  };

  let hook = store
    .create_webhook(room.room_id().as_str(), event.sender.as_str())
    .await?;

  let hook_url = format!(
    "{}api/v1/matrix/hook/{}",
    &config.web.hook_url_base, &hook.id
  );

  admin_room
    .send(
      AnyMessageEventContent::RoomMessage(MessageEventContent::notice_html(
        format!(
          r#"
Here's your webhook url: {url}
To send a message, POST the following JSON to that URL:
{{
  "text": "Hello world!",
  "format": "plain",
  "displayName": "My Cool Webhook",
  "avatarUrl": "{avatar_url}"
}}
"#,
          url = &hook_url,
          avatar_url = &config.webhook_bot.appearance.avatar_url
        ),
        format!(
          r#"Here's your webhook url: <a href="{url}">{url}</a><br>
To send a message, POST the following JSON to that URL:
<pre><code>{{
  "text": "Hello world!",
  "format": "plain",
  "displayName": "My Cool Webhook",
  "avatarUrl": "{avatar_url}"
}}</code></pre>
"#,
          url = &hook_url,
          avatar_url = &config.webhook_bot.appearance.avatar_url
        ),
      )),
      None,
    )
    .await
    .context("Failed to send admin room message")?;

  if let Room::Joined(room) = room {
    room
      .send(
        AnyMessageEventContent::RoomMessage(MessageEventContent::notice_plain(
          "I've sent you a private message with your hook information",
        )),
        None,
      )
      .await
      .context("Failed to send private message notification")?;
  }
  Ok(())
}

async fn handle_room_member_inner(
  config: Arc<config::Config>,
  appservice: AppService,
  room: Room,
  event: SyncStateEvent<MemberEventContent>,
) -> anyhow::Result<()> {
  if event.content.membership != MembershipState::Invite {
    return Ok(());
  }
  let target_user_id = match UserId::try_from(event.state_key) {
    Ok(id) => id,
    Err(_) => return Ok(()),
  };
  let homeserver = <&ServerName>::try_from(config.homeserver.domain.as_str())?;
  let bot_user_id =
    UserId::parse_with_server_name(config.webhook_bot.localpart.as_str(), homeserver)?;
  if target_user_id != bot_user_id {
    debug!("Ignoring invite that is not for the webhook bot");
    return Ok(());
  }
  info!(
    "Received invite to room {}. Joining",
    room.room_id().to_string()
  );

  let client = appservice
    .virtual_user_client(&config.webhook_bot.localpart)
    .await?;
  client.join_room_by_id(room.room_id()).await?;

  Ok(())
}

async fn download_avatar(url: &str) -> anyhow::Result<(mime::Mime, Vec<u8>)> {
  let response = reqwest::get(url)
    .await
    .context("Failed to fetch avatar from provided url")?;

  let response = response.error_for_status()?;
  let mime_raw = match response.headers().get(reqwest::header::CONTENT_TYPE) {
    Some(mime) => mime,
    None => Err(anyhow!("Server did not return a Content-Type header"))?,
  };

  let mime: mime::Mime = mime_raw
    .to_str()
    .context("Failed to convert Content-Type to a string")?
    .parse()
    .context("Could not parse Content-Type into a mime type")?;

  let body = response.bytes().await?;
  if body.len() <= 0 {
    return Err(anyhow!("Avatar request returned empty"))?;
  }

  Ok((mime, body.to_vec()))
}

async fn get_or_create_admin_room(
  client: &Client,
  counterparty: &UserId,
) -> anyhow::Result<RoomId> {
  for room in client.joined_rooms() {
    if room.is_public() {
      trace!("Skipping {} because it is public", room.room_id());
      continue;
    }

    let members = match room.joined_members_no_sync().await {
      Ok(members) => members,
      Err(_) => {
        trace!(
          "Skipping {} because I could not get the members",
          room.room_id()
        );
        continue;
      }
    };

    if members.len() > 2 {
      trace!(
        "Skipping {} because it has {} members",
        room.room_id(),
        members.len()
      );
      continue;
    }

    for member in &members {
      if member.user_id() == counterparty {
        debug!(
          "Using room {} for admin room with {}",
          room.room_id(),
          counterparty
        );
        return Ok(room.room_id().clone());
      }
    }
    trace!(
      "Skipping {} because it doens't have our counterparty {}",
      room.room_id(),
      counterparty
    );
  }

  let invites = vec![counterparty.clone()];
  let mut request = CreateRoomRequest::new();
  request.invite = &invites;
  request.preset = Some(RoomPreset::PrivateChat);
  Ok(client.create_room(request).await?.room_id)
}
