use std::collections::HashMap;

use lazy_static::lazy_static;

// From https://raw.githubusercontent.com/omnidan/node-emoji/master/lib/emoji.json
lazy_static! {
  static ref EMOJI: HashMap<String, String> =
    serde_json::from_str(include_str!("emoji.json")).unwrap();
}

pub fn replace_emoji(s: &str) -> String {
  let mut parts: Vec<String> = s.split(':').map(|s| s.to_owned()).collect();
  let mut out = vec![];

  let num_parts = parts.len();
  let mut skip = false;
  for (i, part) in parts.iter_mut().enumerate() {
    if i == 0 || i == num_parts || skip {
      if i != 0 && !skip {
        out.push(":");
      }
      out.push(part);
      skip = false;
      continue;
    }

    if let Some(replace) = EMOJI.get(part) {
      out.push(replace);
      skip = true;
    } else {
      out.push(":");
      out.push(part);
    }
  }

  out.join("")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_basic() {
    assert_eq!(replace_emoji(":"), ":");
    assert_eq!(replace_emoji(":heart:"), "❤️");
    assert_eq!(replace_emoji("::heart:heart:"), ":❤️heart:");
    assert_eq!(replace_emoji(":heart:heart:"), "❤️heart:");
    assert_eq!(replace_emoji(":heart:::::heart:"), "❤️:::❤️");
    assert_eq!(replace_emoji(":sdfsdfsdfs::heart:"), ":sdfsdfsdfs:❤️");
  }
}
