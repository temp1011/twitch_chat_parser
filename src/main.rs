extern crate irc;
extern crate serde_derive;

use irc::client::prelude::*;
use irc::error::IrcError;
use irc::proto::message::Tag;

use serde_derive::{Deserialize, Serialize};

//https://dev.twitch.tv/docs/irc/tags/#privmsg-twitch-tags
//deprecated tags not serialised
//TODO - optional fields?
#[derive(Debug, Serialize, Deserialize, Default)]
struct TwitchTags {
    badge_info: String,
    badges: Vec<String>,
    bits: u64,      //0 if not bits message
    colour: String, //hex rgb
    display_name: String,
    emotes: Vec<String>,
    id: String,
    moderator: bool,
    room_id: u64,
    tmi_sent_ts: u64, //safe for 2038 problem
    user_id: String,
}

fn s_or_empty(s: Option<String>) -> String {
    match s {
        Some(v) => v,
        None => String::from(""),
    }
}

fn i_or_zero(s: Option<String>) -> u64 {
    match s {
        Some(i) => i.parse::<u64>().unwrap_or(0),
        _ => 0,
    }
}

//TODO - don't bother setting field again if option not present, not sure best way to do this
//if struct stores options can do this with option::map
fn get_tags_struct(tags: Vec<Tag>) -> TwitchTags {
    let mut ret: TwitchTags = Default::default();
    for t in tags.into_iter() {
        match t.0.as_str() {
            "badge-info" => ret.badge_info = s_or_empty(t.1),
            "badges" => {
                ret.badges = s_or_empty(t.1)
                    .split(',')
                    .map(String::from)
                    .collect()
            }
            "bits" => ret.bits = i_or_zero(t.1),
            "color" => ret.colour = s_or_empty(t.1),
            "display-name" => ret.display_name = s_or_empty(t.1),
            "emotes" => {
                ret.emotes = s_or_empty(t.1)
                    .split('/')
                    .map(String::from)
                    .collect()
            }
            "id" => ret.id = s_or_empty(t.1),
            "mod" => ret.moderator = i_or_zero(t.1) != 0,
            "room-id" => ret.room_id = i_or_zero(t.1),
            "tmi-sent-ts" => ret.tmi_sent_ts = i_or_zero(t.1),
            "user-id" => ret.user_id = s_or_empty(t.1),
            _ => {}
        }
    }
    ret
}

fn main() -> Result<(), IrcError> {
    let config = Config::load("config.toml")?;

    let mut reactor = IrcReactor::new()?;
    let client = setup_client(config, &mut reactor)?;

    reactor.register_client_with_handler(client, |client, message| {
        match message.command {
            Command::PRIVMSG(ref target, ref msg) => {
                let tags = match message.tags {
                    Some(t) => get_tags_struct(t),
                    _ => Default::default(),
                };
                println!("{}, {}, {:?}", msg, target, tags);
            }
            Command::PING(_, msg) => {
                client.send_pong(msg.unwrap_or_else(|| String::from("")))?;
            }
            Command::JOIN(ref chan, _, _) => println!("joined {}", chan),
            _ => {} //dbg!(message.command)
        }
        Ok(())
    });

    reactor.run()
}

fn setup_client(config: Config, reactor: &mut IrcReactor) -> Result<IrcClient, IrcError> {
    let client = reactor.prepare_client_and_connect(&config)?;
    client.send_cap_req(&[irc::proto::caps::Capability::Custom("twitch.tv/tags")])?;
    client.identify()?;
    Ok(client)
}
