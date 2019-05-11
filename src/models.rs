use super::schema::messages;

#[derive(Queryable, Insertable)]
#[table_name = "messages"]
pub struct Message {
    pub id: String,
    pub badge_info: Option<String>,
    pub badges: Option<String>,
    pub bits: Option<i32>,
    pub colour: Option<String>,
    pub display_name: Option<String>,
    pub emotes: Option<String>,
    pub moderator: Option<bool>,
    pub room_id: Option<i32>,
    pub tmi_sent_ts: Option<String>,
    pub user_id: Option<String>,
    pub channel: String,
    pub message: String,
    pub raw_message: String,
}
