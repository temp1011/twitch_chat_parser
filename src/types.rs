use std::convert::TryFrom;

use serde::{Deserialize, Serialize};

use chrono::prelude::{DateTime, Utc};

use crate::error::MyError;
use std::time::{Duration, UNIX_EPOCH};

use uuid::Uuid;

use std::sync::Arc;
use twitchchat::{messages::Privmsg, Tags};
//https://dev.twitch.tv/docs/irc/tags/#privmsg-twitch-tags
//deprecated tags not serialised
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
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

//TODO recheck if there's any interesting fields to add to this

impl TryFrom<Tags<'_>> for TwitchTags {
    type Error = MyError;

    //TODO errors as always
    fn try_from(tags: Tags) -> Result<Self, Self::Error> {
        //TODO can probably just use turbofish type for this instead of separate param
        let badges: Option<String> = tags.get_parsed("badges");
        let emotes: Option<String> = tags.get_parsed("emotes");
        let uuidstr: Option<String> = tags.get_parsed("id");
        let timestr: Option<String> = tags.get_parsed("tmi-sent-ts");
        Ok(TwitchTags {
            badge_info: tags.get_parsed("badge-info"),
            badges: badges.map(|s| s.split(',').map(String::from).collect()),
            bits: tags.get_parsed("bits"),
            color: tags.get_parsed("color"), //https://en.wikipedia.org/wiki/Web_colors
            display_name: tags
                .get_parsed("display-name")
                .ok_or(MyError::Parse("Display name not present"))?,
            emotes: emotes.map(|s| s.split('/').map(String::from).collect()),
            id: uuidstr
                .as_ref()
                .and_then(|s| Uuid::parse_str(s).ok())
                .ok_or(MyError::Parse("Message id not present"))?,
            moderator: tags.get_parsed("mod"),
            room_id: tags
                .get_parsed("room-id")
                .ok_or(MyError::Parse("room id not present"))?,
            tmi_sent_ts: timestr
                .ok_or(MyError::Parse("Timestamp not present"))?
                .parse::<u64>()
                .map_err(|_| MyError::Parse("timestamp wasn't a u64"))
                .map(|v| DateTime::<Utc>::from(UNIX_EPOCH + Duration::from_millis(v)))?,
            user_id: tags
                .get_parsed("user-id")
                .ok_or(MyError::Parse("User id not present"))?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct TwitchMessage {
    pub tags: TwitchTags,
    pub channel: String,
    pub message: String,
    pub raw: String,
}

impl TryFrom<Arc<Privmsg<'_>>> for TwitchMessage {
    type Error = MyError;

    //TODO can get a lot of stuff from message directly may not need tags as much
    fn try_from(msg: Arc<Privmsg>) -> Result<Self, Self::Error> {
        let raw_msg: String = format!("{:?}", msg.clone());
        Ok(TwitchMessage {
            tags: TwitchTags::try_from(msg.tags.clone())?,
            channel: msg.channel.to_string(),
            message: msg.data.to_string(),
            raw: raw_msg,
        })
    }
}
