use irc::client::prelude::*;
use irc::proto::message::Tag;

use serde_derive::{Deserialize, Serialize};

use chrono::prelude::{DateTime, Utc};
use chrono::TimeZone;

//https://dev.twitch.tv/docs/irc/tags/#privmsg-twitch-tags
//deprecated tags not serialised
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TwitchTags {
    pub badge_info: Option<String>,
    pub badges: Option<Vec<String>>,
    pub bits: Option<i32>,
    pub colour: Option<String>, //hex rgb
    pub display_name: Option<String>,
    pub emotes: Option<Vec<String>>,
    pub id: String, //probably https://www.ietf.org/rfc/rfc4122.txt
    pub moderator: Option<bool>,
    pub room_id: Option<i32>,

    //#[serde(with = "ts_milliseconds")] possible with custom deserializer
    pub tmi_sent_ts: Option<DateTime<Utc>>,
    pub user_id: Option<String>,
}

impl TwitchTags {
    //TODO - throw on more or maybe deserializer
    fn from_irc_tags(tags: Vec<Tag>) -> Result<TwitchTags, &'static str> {
        let mut ret: TwitchTags = Default::default();
        for t in tags.into_iter() {
            let val = t.1.filter(|x| !x.is_empty());
            match t.0.as_str() {
                "badge-info" => ret.badge_info = val,
                "badges" => ret.badges = val.map(|s| s.split(',').map(String::from).collect()),
                "bits" => ret.bits = val.map(map_to_int),
                "color" => ret.colour = val,
                "display-name" => ret.display_name = val,
                "emotes" => ret.emotes = val.map(|s| s.split('/').map(String::from).collect()),
                "id" => {
                    ret.id = match val {
                        Some(i) => i,
                        None => {
                            return Err("id not present");
                        }
                    }
                }
                "mod" => ret.moderator = val.map(map_to_int).map(|i| i != 0),
                "room-id" => ret.room_id = val.map(map_to_int),
                "tmi-sent-ts" => {
                    //very ugly atm, simplify with format string?
                    ret.tmi_sent_ts = val
                        .map(|s| s.parse::<u64>().unwrap_or(0))
                        .map(|v| Utc.timestamp((v / 1000) as i64, ((v % 1000) * 1_000_000) as u32))
                }
                "user-id" => ret.user_id = val,
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

impl TwitchMessage {
    pub fn from_irc_message(
        message: &Message,
        target: &str,
        text: &str,
    ) -> Result<TwitchMessage, &'static str> {
        let orig = message.to_string();

        let tgs = match &message.tags {
            Some(t) => match TwitchTags::from_irc_tags(t.to_vec()) {
                Ok(r) => r,
                Err(e) => {
                    return Err(e);
                }
            },
            _ => return Err("no tags present in message"),
        };

        Ok(TwitchMessage {
            tags: tgs,
            channel: target.to_string(),
            message: text.to_string(),
            raw: orig,
        })
    }
}
