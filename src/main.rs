#[macro_use]
extern crate diesel;

pub mod db;
pub mod models;
pub mod schema;
use irc::client::prelude::*;
use irc::error::IrcError;

use std::io::{Error, ErrorKind};

mod types;
use types::TwitchMessage;

mod channels;

const MAX_CHANNELS: u64 = 20;

//TODO - IrcError doesn't have from Box<Error>, so how to handle multiple types?
//it has inner field containing error itself. Not sure how to wrap this to include normal errors
//too. The error handling here is probably too lax anyway.
//
//my errors here are awful...
fn main() -> Result<(), IrcError> {
    let mut reactor = IrcReactor::new()?;
    let client = setup_client(&mut reactor)?;
    let conn = db::DB::connection().unwrap();
    reactor.register_client_with_handler(client, move |client, message| {
        match message.command {
            Command::PRIVMSG(ref target, ref msg) => {
                let t_msg = match TwitchMessage::from_irc_message(&message, target, msg) {
                    Ok(t) => t,
                    Err(e) => return Err(IrcError::Io(Error::new(ErrorKind::Other, e))),
                };
                if let Err(e) = conn.send(t_msg) {
                    Error::new(ErrorKind::Other, e);
                }
            }
            Command::PING(_, msg) => {
                client.send_pong(msg.unwrap_or_else(String::new))?;
            }
            Command::JOIN(ref chan, _, _) => println!("joined {}", chan),
            _ => {}
        }
        Ok(())
    });

    reactor.run()?;
    Ok(())
}

fn setup_client(reactor: &mut IrcReactor) -> Result<IrcClient, IrcError> {
    let mut config = Config::load("config.toml")?;
    let mut nick = "justinfan".to_string();
    nick.push_str(&rand::random::<u16>().to_string());
    config.nickname = Some(nick);

    config.server = Some("irc.chat.twitch.tv".to_string());

    //TODO - how to custom config?
    //    let config_number_channels: u64 = config.number_channels.unwrap_or(100);
    let mut config_channels = config.channels.unwrap_or_default();
    let mut top_channels =
        channels::top_connections(MAX_CHANNELS.saturating_sub(config_channels.len() as u64));
    config_channels.append(&mut top_channels);
    config_channels.dedup();
    config.channels = Some(config_channels);

    let client = reactor.prepare_client_and_connect(&config)?;
    client.send_cap_req(&[irc::proto::caps::Capability::Custom("twitch.tv/tags")])?;
    client.identify()?;
    Ok(client)
}
