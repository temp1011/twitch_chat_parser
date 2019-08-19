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
use types::TwitchMessage;
use error::MyError;
mod channels;
mod error;
mod controller;
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
    while let Ok(msg) = msg_recv.recv() {   //TODO for msg in msg_recv?
        if let Err(e) = db_conn.send(msg) {
            return ();//Err(MyError::Db(Box::new(e))); //TODO need 'other' error type
        }
    }

    ()
    });

    loop {
        thread::sleep(Duration::from_secs(2));
        println!("{:?}", controller.list());
    }
}
