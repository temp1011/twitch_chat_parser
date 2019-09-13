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
use std::time::Duration;

mod types;
use error::MyError;
use types::TwitchMessage;
mod channels;
mod controller;
mod error;
use controller::IrcController;
const MAX_CHANNELS: u64 = 300;

//TODO - IrcError doesn't have from Box<Error>, so how to handle multiple types?
//it has inner field containing error itself. Not sure how to wrap this to include normal errors
//too. The error handling here is probably too lax anyway.
//
//my errors here are awful...
fn main() -> Result<(), error::MyError> {
    let db_conn = db::DB::connection().unwrap();

    //TODO use init with sender
    let (msg_recv, mut controller) = IrcController::init(channels::top_connections(MAX_CHANNELS))?;
    //Tie together the channels
    let handle = thread::spawn(move || {
        while let Ok(msg) = msg_recv.recv() {
            //TODO for msg in msg_recv?
            if let Err(e) = db_conn.send(msg) {
                return (); //Err(MyError::Db(Box::new(e))); //TODO need 'other' error type
            }
        }

        ()
    });

    //NOTE: crashes, same issue as https://github.com/aatxe/irc/issues/174, possible solution is use more reactors with fewer channels each
    //TODO load balancing: have y total channels and x reactors. For each figure out too leave,
    //then append to join to get back to original capacity.
    loop {
        thread::sleep(Duration::from_secs(30));
        let current_channels = controller.list();
        let top_channels = channels::top_connections(MAX_CHANNELS);
        //TODO potentially n^2? Solvable with hashmap
        //TODO see if clones necessary too
        let to_join: Vec<String> = top_channels
            .clone()
            .into_iter()
            .filter(|c| !current_channels.contains(c))
            .collect();
        let to_part: Vec<String> = current_channels
            .clone()
            .into_iter()
            .filter(|c| !top_channels.contains(c))
            .collect();
        for c in to_join {
            if let Err(e) = controller.join(c) {
                //TODO handle result
                println!("{:?}", e);
            }
        }
        for c in to_part {
            if let Err(e) = controller.part(c) {
                println!("{:?}", e);
            }
        }
    }
}
