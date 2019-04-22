use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dotenv::dotenv;
use std::env;
use crate::schema::messages;
use crate::models::NewMessage;
use crate::types::TwitchMessage;
pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
        SqliteConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

pub fn insert(conn: &SqliteConnection, message: &mut TwitchMessage) -> QueryResult<usize> {
    let message = message.clone();
    let db_message = NewMessage {
        badge_info: message.tags.badge_info,
        badges: message.tags.badges.map(vec_to_json),
        bits: message.tags.bits,
        colour: message.tags.colour,
        display_name: message.tags.display_name,
        emotes: message.tags.emotes.map(vec_to_json),
        message_id: message.tags.id,
        moderator: message.tags.moderator,
        room_id: message.tags.room_id,
        tmi_sent_ts: message.tags.tmi_sent_ts.map(|d| d.to_string()),
        user_id: message.tags.user_id,
        channel: message.channel,
        message: message.message,
        raw_message: message.raw
    };
    
    diesel::insert_into(messages::table)
        .values(&db_message)
        .execute(conn)
}

//TODO - where T: Deserialize?
fn vec_to_json(v: Vec<String>) -> String {
    serde_json::to_string(&v).unwrap()
}
