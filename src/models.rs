use super::schema::messages;
use crate::types::TwitchMessage;

#[derive(Insertable)]
pub struct Message {
    pub id: String,
    pub badge_info: Option<String>,
    pub badges: Option<String>,
    pub bits: Option<i32>,
    pub color: Option<String>,
    pub display_name: Option<String>,
    pub emotes: Option<String>,
    pub mod_: Option<bool>,
    pub room_id: Option<i32>,
    pub tmi_sent_ts: Option<String>,
    pub user_id: Option<String>,
    pub channel: String,
    pub message: String,
    pub raw_message: String,
}

impl From<TwitchMessage> for Message {
    fn from(message: TwitchMessage) -> Self {
        Message {
            id: message.tags.id,
            badge_info: message.tags.badge_info,
            badges: message.tags.badges.map(vec_to_json),
            bits: message.tags.bits,
            color: message.tags.color,
            display_name: message.tags.display_name,
            emotes: message.tags.emotes.map(vec_to_json),
            mod_: message.tags.moderator,
            room_id: message.tags.room_id,
            tmi_sent_ts: message.tags.tmi_sent_ts.map(|d| d.to_string()),
            user_id: message.tags.user_id,
            channel: message.channel,
            message: message.message,
            raw_message: message.raw.trim().to_string(),
        }
    }
}

fn vec_to_json<T: serde::Serialize>(v: Vec<T>) -> String {
    serde_json::to_string(&v).unwrap_or_default()
}
