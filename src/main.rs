#[macro_use]
extern crate diesel;

pub mod db;
pub mod models;
pub mod schema;
use irc::client::prelude::*;
use irc::error::IrcError;

use std::sync::mpsc::channel;
use std::thread;

use std::io::{Error, ErrorKind};

mod types;
use types::TwitchMessage;

//TODO - IrcError doesn't have from Box<Error>, so how to handle multiple types?
//it has inner field containing error itself. Not sure how to wrap this to include normal errors
//too. The error handling here is probably too lax anyway.
//
//my errors here are awful...
fn main() -> Result<(), IrcError> {
    let config = Config::load("config.toml")?;

    let mut reactor = IrcReactor::new()?;
    let client = setup_client(config, &mut reactor)?;

    let (tx, rx) = channel::<TwitchMessage>();

    let thread = thread::spawn(move || {
        let conn = db::establish_connection();
        while let Ok(mut v) = rx.recv() {
            println!("{}", serde_json::to_string(&v).unwrap());
            if let Err(e) = db::insert(&conn, &mut v) {
                println!("{:?}", e);
            }
        }
    });

    reactor.register_client_with_handler(client, move |client, message| {
        match message.command {
            Command::PRIVMSG(ref target, ref msg) => {
                let t_msg = match TwitchMessage::from_irc_message(&message, target, msg) {
                    Ok(t) => t,
                    Err(e) => return Err(IrcError::Io(Error::new(ErrorKind::Other, e))),
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
