use crate::emoji;
use matrix_sdk::ruma::events::room::message::{
  EmoteMessageEventContent, MessageEventContent, MessageType,
};
use serde::Deserialize;

#[derive(Debug, PartialEq, Deserialize)]
pub struct WebhookRequest {
  text: String,
  format: Format,
  #[serde(rename = "displayName")]
  pub display_name: String,
  #[serde(rename = "avatarUrl")]
  pub avatar_url: String,
  #[serde(default = "return_true")]
  emoji: bool,
  #[serde(default, rename = "msgtype")]
  message_type: MsgType,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Format {
  Plain,
  Html,
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum MsgType {
  Regular,
  Notice,
  Emote,
}

impl Default for MsgType {
  fn default() -> Self {
    Self::Regular
  }
}

fn return_true() -> bool {
  true
}

impl WebhookRequest {
  pub fn create_message(&self) -> MessageEventContent {
    use Format::*;
    use MsgType::*;

    let parsed = self.parse_text();
    match (&self.message_type, &self.format) {
      (Regular, Plain) => MessageEventContent::text_plain(parsed),
      (Regular, Html) => MessageEventContent::text_html(Self::html_to_text(&parsed), parsed),
      (Notice, Plain) => MessageEventContent::notice_plain(parsed),
      (Notice, Html) => MessageEventContent::notice_html(Self::html_to_text(&parsed), parsed),
      (Emote, Plain) => {
        MessageEventContent::new(MessageType::Emote(EmoteMessageEventContent::plain(parsed)))
      }
      (Emote, Html) => MessageEventContent::new(MessageType::Emote(
        EmoteMessageEventContent::html(Self::html_to_text(&parsed), parsed),
      )),
    }
  }

  fn parse_text(&self) -> String {
    if self.emoji {
      emoji::replace_emoji(&self.text)
    } else {
      self.text.clone()
    }
  }

  fn html_to_text(raw: &str) -> String {
    let frag = scraper::Html::parse_fragment(raw);
    frag
      .tree
      .into_iter()
      .filter_map(|node| {
        if let scraper::node::Node::Text(text) = node {
          Some(text.text.to_string())
        } else {
          None
        }
      })
      .collect::<Vec<String>>()
      .join("")
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use anyhow::Result;
  use matrix_sdk::ruma::events::room::message::MessageType;

  #[test]
  fn test_basic() -> Result<()> {
    let raw_json = r#"
     {
       "text": "Hello world!",
       "format": "plain",
       "displayName": "My Cool Webhook",
       "avatarUrl": "http://i.imgur.com/IDOBtEJ.png"
     }"#;

    let expected = WebhookRequest {
      text: "Hello world!".into(),
      format: Format::Plain,
      display_name: "My Cool Webhook".into(),
      avatar_url: "http://i.imgur.com/IDOBtEJ.png".into(),
      emoji: true,
      message_type: MsgType::Regular,
    };

    let parsed = serde_json::from_str::<WebhookRequest>(raw_json)?;
    assert_eq!(parsed, expected);

    let expected_message_body = "Hello world!";

    if let MessageType::Text(actual_message) = expected.create_message().msgtype {
      assert_eq!(expected_message_body, actual_message.body);
    } else {
      panic!("Not text");
    }

    Ok(())
  }

  #[test]
  fn test_html() -> Result<()> {
    let raw_json = r#"
    {
      "text": "<b>Hello world!</b> <br><ol><li>aa</li> <li>bb</li></ol>",
      "format": "html",
      "displayName": "My Cool Webhook",
      "avatarUrl": "https://i.imgur.com/IDOBtEJ.png"
  }"#;

    let parsed = serde_json::from_str::<WebhookRequest>(raw_json)?;
    let actual = if let MessageType::Text(actual_message) = parsed.create_message().msgtype {
      actual_message
    } else {
      panic!("Not text");
    };

    let formatted = actual.formatted.unwrap();
    assert_eq!(formatted.format.as_str(), "org.matrix.custom.html");
    assert_eq!(actual.body, "Hello world! aa bb");
    assert_eq!(
      formatted.body,
      "<b>Hello world!</b> <br><ol><li>aa</li> <li>bb</li></ol>"
    );

    Ok(())
  }

  #[test]
  fn test_notice_html() -> Result<()> {
    let raw_json = r#"
    {
      "text": "<b>Hello world!</b> <br><ol><li>aa</li> <li>bb</li></ol>",
      "format": "html",
      "msgtype": "notice",
      "displayName": "My Cool Webhook",
      "avatarUrl": "https://i.imgur.com/IDOBtEJ.png"
  }"#;

    let parsed = serde_json::from_str::<WebhookRequest>(raw_json)?;
    let actual = if let MessageType::Notice(actual_message) = parsed.create_message().msgtype {
      actual_message
    } else {
      panic!("Not notice");
    };

    let formatted = actual.formatted.unwrap();
    assert_eq!(formatted.format.as_str(), "org.matrix.custom.html");
    assert_eq!(actual.body, "Hello world! aa bb");
    assert_eq!(
      formatted.body,
      "<b>Hello world!</b> <br><ol><li>aa</li> <li>bb</li></ol>"
    );

    Ok(())
  }

  #[test]
  fn test_emote_html() -> Result<()> {
    let raw_json = r#"
    {
      "text": "<b>Hello world!</b> <br><ol><li>aa</li> <li>bb</li></ol>",
      "format": "html",
      "msgtype": "emote",
      "displayName": "My Cool Webhook",
      "avatarUrl": "https://i.imgur.com/IDOBtEJ.png"
  }"#;

    let parsed = serde_json::from_str::<WebhookRequest>(raw_json)?;
    let actual = if let MessageType::Emote(actual_message) = parsed.create_message().msgtype {
      actual_message
    } else {
      panic!("Not notice");
    };

    let formatted = actual.formatted.unwrap();
    assert_eq!(formatted.format.as_str(), "org.matrix.custom.html");
    assert_eq!(actual.body, "Hello world! aa bb");
    assert_eq!(
      formatted.body,
      "<b>Hello world!</b> <br><ol><li>aa</li> <li>bb</li></ol>"
    );

    Ok(())
  }

  #[test]
  fn test_emoji_html() -> Result<()> {
    let raw_json = r#"
    {
      "text": "<b>foo:heart::heart:</b> <br><ol><li>aa</li> <li>bb</li></ol>",
      "format": "html",
      "msgtype": "emote",
      "displayName": "My Cool Webhook",
      "avatarUrl": "https://i.imgur.com/IDOBtEJ.png"
  }"#;

    let parsed = serde_json::from_str::<WebhookRequest>(raw_json)?;
    let actual = if let MessageType::Emote(actual_message) = parsed.create_message().msgtype {
      actual_message
    } else {
      panic!("Not notice");
    };

    let formatted = actual.formatted.unwrap();
    assert_eq!(formatted.format.as_str(), "org.matrix.custom.html");
    assert_eq!(actual.body, "foo❤️❤️ aa bb");
    assert_eq!(
      formatted.body,
      "<b>foo❤️❤️</b> <br><ol><li>aa</li> <li>bb</li></ol>"
    );

    Ok(())
  }
}
