use crate::models::Message;
use crate::schema::messages;
use crate::types::TwitchMessage;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dotenv::dotenv;
use std::env;

pub fn establish_connection() -> ConnectionResult<SqliteConnection> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
}

pub fn insert(conn: &SqliteConnection, message: TwitchMessage) -> QueryResult<usize> {
    let db_message = Message {
        id: message.tags.id.unwrap(),
        badge_info: message.tags.badge_info,
        badges: message.tags.badges.map(vec_to_json),
        bits: message.tags.bits,
        colour: message.tags.colour,
        display_name: message.tags.display_name,
        emotes: message.tags.emotes.map(vec_to_json),
        moderator: message.tags.moderator,
        room_id: message.tags.room_id,
        tmi_sent_ts: message.tags.tmi_sent_ts.map(|d| d.to_string()),
        user_id: message.tags.user_id,
        channel: message.channel,
        message: message.message,
        raw_message: message.raw.trim().to_string(),
    };

    diesel::insert_into(messages::table)
        .values(&db_message)
        .execute(conn)
}

fn vec_to_json<T: serde::Serialize>(v: Vec<T>) -> String {
    serde_json::to_string(&v).unwrap()
}
