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
use controller::Controller;
use controller::IrcController;
use std::cmp;
use std::sync::mpsc::*;
use std::collections::HashSet;
use std::iter::FromIterator;
use rand::{thread_rng, Rng};

const MAX_CHANNELS: u64 = 300;
const CHANNELS_PER_CONTROLLER: u64 = 30;

//TODO - IrcError doesn't have from Box<Error>, so how to handle multiple types?
//it has inner field containing error itself. Not sure how to wrap this to include normal errors
//too. The error handling here is probably too lax anyway.
//
//my errors here are awful...
fn main() -> Result<(), error::MyError> {
    let db_conn: Sender<TwitchMessage> = db::DB::connection().unwrap();
    //TODO might want this as a set, but maybe can't do chunks then?
    let mut chans = channels::top_connections(MAX_CHANNELS);
    //provide more even load of channels between controllers
    thread_rng().shuffle(&mut chans);
    //This could/should be Vec<Set>
    let chans_split: Vec<Vec<String>> = chans.chunks(CHANNELS_PER_CONTROLLER as usize).map(|c| c.to_vec()).collect();
    let controllers: Vec<Controller> = chans_split
        .into_iter()
        .map(|s| Controller::init_with_sender(s, db_conn.clone()).unwrap())
        .collect();

    //NOTE: crashes, same issue as https://github.com/aatxe/irc/issues/174, possible solution is use more reactors with fewer channels each
    //TODO load balancing: have y total channels and x reactors. For each figure out to leave,
    //then append to join to get back to original capacity.
    loop {
        thread::sleep(Duration::from_secs(30));
        let mut top_channels: HashSet<String> = HashSet::from_iter(channels::top_connections(MAX_CHANNELS).into_iter());
        
        let mut temp: Vec<(&Controller, Vec<String>)> = controllers.iter().zip(Vec::new()).collect();
        for (c, v) in &mut temp {
            for l in c.list() {
                if top_channels.remove(&l) {
                    v.push(l);
                }
            }
        }
        
        let mut it = top_channels.drain();  //To join
        for(c, v) in &mut temp {
            if v.is_empty() {
                continue;
            }
            for to_leave in v {
                c.join(it.next().unwrap());
                c.part(to_leave.to_string());
            }
        }
    }
}
