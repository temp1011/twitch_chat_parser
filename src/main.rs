extern crate chrono;
extern crate irc;
extern crate serde_derive;
extern crate serde_json;
extern crate serde_with_macros;

use irc::client::prelude::*;
use irc::error::IrcError;
use irc::proto::message::Tag;

use serde_derive::{Deserialize, Serialize};
use serde_with_macros::skip_serializing_none;

use std::sync::mpsc::channel;
use std::thread;

use std::io::{Error, ErrorKind};

use chrono::prelude::{DateTime, Utc};
use chrono::TimeZone;

//https://dev.twitch.tv/docs/irc/tags/#privmsg-twitch-tags
//deprecated tags not serialised
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Default)]
struct TwitchTags {
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

#[derive(Debug, Serialize, Deserialize)]
struct TwitchMessage {
    tags: TwitchTags,
    channel: String,
    message: String,
    raw: String,
}

fn map_to_int(s: String) -> u64 {
    s.parse::<u64>().unwrap_or(0)
}

fn get_tags_struct(tags: Vec<Tag>) -> TwitchTags {
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

//TODO - IrcError doesn't have from Box<Error>, so how to handle multiple types?
//it has inner field containing error itself. Not sure how to wrap this to include normal errors
//too. The error handlnig here is probably too lax anyway.
fn main() -> Result<(), IrcError> {
    let config = Config::load("config.toml")?;

    let mut reactor = IrcReactor::new()?;
    let client = setup_client(config, &mut reactor)?;

    let (tx, rx) = channel::<TwitchMessage>();

    let thread = thread::spawn(move || {
        while let Ok(v) = rx.recv() {
            println!("{}", serde_json::to_string(&v).unwrap());
        }
    });

    reactor.register_client_with_handler(client, move |client, message| {
        match message.command {
            Command::PRIVMSG(ref target, ref msg) => {
                let orig = message.to_string();

                let tgs = match message.tags {
                    Some(t) => get_tags_struct(t),
                    _ => Default::default(),
                };

                let t_msg = TwitchMessage {
                    tags: tgs,
                    channel: target.to_string(),
                    message: msg.to_string(),
                    raw: orig,
                };
                if let Err(e) = tx.clone().send(t_msg) {
                    Error::new(ErrorKind::Other, e);
                }
            }
            Command::PING(_, msg) => {
                client.send_pong(msg.unwrap_or_else(String::new))?;
            }
            //Command::JOIN(ref chan, _, _) => println!("joined {}", chan),
            _ => {}
        }
        Ok(())
    });

    reactor.run()?;
    if let Err(e) = thread.join() {
        println!("receiver panicked, {:?}", e);
    }
    Ok(())
}

fn setup_client(config: Config, reactor: &mut IrcReactor) -> Result<IrcClient, IrcError> {
    let client = reactor.prepare_client_and_connect(&config)?;
    client.send_cap_req(&[irc::proto::caps::Capability::Custom("twitch.tv/tags")])?;
    client.identify()?;
    Ok(client)
}
