#[macro_use]
extern crate diesel;

mod db;
mod models;
mod schema;
use irc::client::prelude::*;
use irc::error::IrcError;

use std::convert::TryFrom;
use std::io::{Error, ErrorKind};
use std::sync::mpsc::{channel, Receiver};
use std::thread;

mod types;
use types::TwitchMessage;
use error::MyError;
mod channels;
mod error;
mod videos;
mod controller2;
const MAX_CHANNELS: u64 = 300;

//TODO - IrcError doesn't have from Box<Error>, so how to handle multiple types?
//it has inner field containing error itself. Not sure how to wrap this to include normal errors
//too. The error handling here is probably too lax anyway.
//
//my errors here are awful...
fn main() -> Result<(), error::MyError> {
    let db_conn = db::DB::connection().unwrap();
    
    //TODO loop? this will exit on first error.
    let msg_recv = run_client()?;
    //Tie together the channels
    while let Ok(msg) = msg_recv.recv() {   //TODO for msg in msg_recv?
        if let Err(e) = db_conn.send(msg) {
            return Err(MyError::Db(Box::new(e))); //TODO need 'other' error type
        }
    }

    Ok(())
}

//TODO need to return join handle?
fn run_client() -> Result<Receiver<TwitchMessage>, IrcError> {
    let (send, recv) = channel::<TwitchMessage>();
   

    thread::spawn(move || {

    let mut reactor = IrcReactor::new().unwrap();   //TODO errors
    let client = setup_client(&mut reactor).expect("Failed to setup client");   //TODO errors
    //TODO - use multiple clients for better parallelism, given that twitch seems to rate limit
    //joining channels.
    reactor.register_client_with_handler(client, move |client, message| {
        if let Ok(t_msg) = TwitchMessage::try_from(&message) {
            if let Err(e) = send.send(t_msg) {
                Error::new(ErrorKind::Other, e);
            }
            return Ok(());
        }

        match message.command {
            Command::PING(_, msg) => {
                client.send_pong(msg.unwrap_or_else(String::new))?;
            }
            Command::JOIN(ref chan, _, _) => println!("joined {}", chan),
            _ => {}
        }
        Ok(())
    });

    reactor.run();
    });
    Ok(recv)

}

fn setup_client(reactor: &mut IrcReactor) -> Result<IrcClient, IrcError> {
    let mut config = Config::load("config.toml")?;
    let mut nick = "justinfan".to_string();
    nick.push_str(&rand::random::<u32>().to_string());

    let config = Config {
        nickname: Some(nick),
        server: Some("irc.chat.twitch.tv".to_owned()),
        channels: Some(channels::top_connections(MAX_CHANNELS)),
        ..Config::default()
    };
    let client = reactor.prepare_client_and_connect(&config)?;
    client.send_cap_req(&[irc::proto::caps::Capability::Custom("twitch.tv/tags")])?;
    client.identify()?;
    Ok(client)
}
