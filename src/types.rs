use std::convert::TryFrom;
use std::str::FromStr;

use irc::client::prelude::*;
use irc::proto::message::Tag;

use serde::{Deserialize, Serialize};

use chrono::prelude::{DateTime, NaiveDateTime, Utc};

use crate::error::MyError;
use std::time::{Duration, UNIX_EPOCH};

use uuid::Uuid;
//https://dev.twitch.tv/docs/irc/tags/#privmsg-twitch-tags
//deprecated tags not serialised
#[derive(Debug, Serialize, Deserialize)]
pub struct TwitchTags {
    pub badge_info: Option<String>,
    pub badges: Option<Vec<String>>,
    pub bits: Option<i32>,

    //https://en.wikipedia.org/wiki/Web_colors
    pub color: Option<String>,
    pub display_name: String,
    pub emotes: Option<Vec<String>>,
    pub id: Uuid,
    pub moderator: Option<bool>,
    pub room_id: i32,
    pub tmi_sent_ts: DateTime<Utc>,
    pub user_id: String,
}

impl Default for TwitchTags {
    fn default() -> Self {
        TwitchTags {
            badge_info: None,
            badges: None,
            bits: None,
            color: None,
            display_name: "".to_string(),
            emotes: None,
            id: Uuid::nil(),
            moderator: None,
            room_id: 0,
            user_id: "".to_string(),
            tmi_sent_ts: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc),
        }
    }
}

impl TryFrom<Vec<Tag>> for TwitchTags {
    type Error = MyError;
    //TODO - throw on more or maybe deserializer
    fn try_from(tags: Vec<Tag>) -> Result<Self, Self::Error> {
        let mut ret: TwitchTags = Default::default();
        for t in tags.into_iter() {
            let val = t.1.filter(|x| !x.is_empty());
            match t.0.as_str() {
                "badge-info" => ret.badge_info = val,
                "badges" => ret.badges = val.map(|s| s.split(',').map(String::from).collect()),
                "bits" => ret.bits = val.map(map_to_int),
                "color" => ret.color = val,
                "display-name" => {
                    ret.display_name = val.ok_or(MyError::Parse("Display name not present"))?
                }
                "emotes" => ret.emotes = val.map(|s| s.split('/').map(String::from).collect()),
                //TODO lose an opputunity to capture the parse error here.
                "id" => {
                    ret.id = val
                        .as_ref()
                        .and_then(|v| Uuid::parse_str(v).ok())
                        .ok_or(MyError::Parse("message id not present"))?
                }
                "mod" => ret.moderator = val.map(map_to_int).map(|i| i != 0),
                "room-id" => {
                    ret.room_id = val
                        .map(map_to_int)
                        .ok_or(MyError::Parse("Room id not present"))?
                }
                "tmi-sent-ts" => {
                    ret.tmi_sent_ts = val
                        .and_then(|s| s.parse::<u64>().ok()) //TODO might be nice to integrate this to error chain too
                        .map(|v| DateTime::<Utc>::from(UNIX_EPOCH + Duration::from_millis(v)))
                        .ok_or(MyError::Parse("Timestamp not present"))?
                }
                "user-id" => ret.user_id = val.ok_or(MyError::Parse("User id not present"))?,
                _ => {}
            }
        }
        Ok(ret)
    }
}

fn map_to_int(s: String) -> i32 {
    s.parse::<i32>().unwrap_or(0)
}
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TwitchMessage {
    pub tags: TwitchTags,
    pub channel: String,
    pub message: String,
    pub raw: String,
}

impl TryFrom<&Message> for TwitchMessage {
    type Error = MyError;

    fn try_from(irc_msg: &Message) -> Result<Self, Self::Error> {
        if let Command::PRIVMSG(ref target, ref msg) = irc_msg.command {
            let orig = irc_msg.to_string();

            //unwrap_or?
            let tgs = match &irc_msg.tags {
                Some(t) => TwitchTags::try_from(t.to_vec())?,
                _ => return Err(MyError::Parse("no tags present in message")),
            };

            Ok(TwitchMessage {
                tags: tgs,
                channel: target.to_string(),
                message: msg.to_string(),
                raw: orig,
            })
        } else {
            Err(MyError::Parse("Not a PRIVMSG"))
        }
    }
}

impl FromStr for TwitchMessage {
    type Err = MyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<Message>() {
            Ok(msg) => TwitchMessage::try_from(&msg),
            Err(e) => Err(MyError::Parse("could not be parsed to irc message")), //should pass actual type here
        }
    }
}
