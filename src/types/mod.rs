//extern crate chrono;
//extern crate irc;
//extern crate serde_derive;
//extern crate serde_json;
extern crate serde_with_macros;

use irc::client::prelude::*;
use irc::proto::message::Tag;

use serde_derive::{Deserialize, Serialize};
use serde_with_macros::skip_serializing_none;

use chrono::prelude::{DateTime, Utc};
use chrono::TimeZone;

//https://dev.twitch.tv/docs/irc/tags/#privmsg-twitch-tags
//deprecated tags not serialised
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TwitchTags {
    badge_info: Option<String>,
    badges: Option<Vec<String>>,
    bits: Option<u64>,
    colour: Option<String>, //hex rgb
    display_name: Option<String>,
    emotes: Option<Vec<String>>,
    id: Option<String>,
    moderator: Option<bool>,
    room_id: Option<u64>,

    //#[serde(with = "ts_milliseconds")] possible with custom deserializer
    tmi_sent_ts: Option<DateTime<Utc>>,
    user_id: Option<String>,
}

impl TwitchTags {
    fn from_irc_tags(tags: Vec<Tag>) -> TwitchTags {
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
                "id" => ret.id = val,
                "mod" => ret.moderator = val.map(map_to_int).map(|i| i != 0),
                "room-id" => ret.room_id = val.map(map_to_int),
                "tmi-sent-ts" => {
                    ret.tmi_sent_ts = val
                        .map(map_to_int)
                        .map(|v| Utc.timestamp((v / 1000) as i64, ((v % 1000) * 1_000_000) as u32))
                }
                "user-id" => ret.user_id = val,
                _ => {}
            }
        }
        ret
    }
}

fn map_to_int(s: String) -> u64 {
    s.parse::<u64>().unwrap_or(0)
}
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TwitchMessage {
    tags: TwitchTags,
    channel: String,
    message: String,
    raw: String,
}

impl TwitchMessage {
    pub fn from_irc_message(
        message: &Message,
        target: &str,
        text: &str,
    ) -> Result<TwitchMessage, &'static str> {
        let orig = message.to_string();

        let tgs = match &message.tags {
            Some(t) => TwitchTags::from_irc_tags(t.to_vec()),
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

//                let t_msg = TwitchMessage {
//                    tags: tgs,
//                    channel: target.to_string(),
//                    message: msg.to_string(),
//                    raw: orig,
//                };
