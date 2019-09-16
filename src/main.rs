#[macro_use]
extern crate diesel;

mod db;
mod models;
mod schema;

use std::thread;
use std::time::Duration;

mod types;
use types::TwitchMessage;
mod channels;
mod controller;
mod error;
use controller::Controller;
use controller::IrcController;
use rand::{thread_rng, Rng};
use std::collections::HashSet;
use std::iter::FromIterator;
use std::sync::mpsc::*;

const MAX_CHANNELS: u64 = 1000;
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
    //either my API usage/understanding is broken or twitch is returning a bad value here
    let mut seen_set = HashSet::<String>::with_capacity(chans.len());
    chans.retain(|c| {
        let seen = !seen_set.insert(c.to_string());
        if seen {
            eprintln!(
                "channel {} was found twice in channels returned by API, removing duplicate",
                c
            );
        }
        !seen
    });
    if chans.len() < MAX_CHANNELS as usize {
        eprintln!(
            "API returned fewer channels than expected. Expected {}, got {}",
            MAX_CHANNELS,
            chans.len()
        );
    }
    assert_eq!(
        chans.len(),
        HashSet::<&String>::from_iter(chans.iter()).len()
    ); //check there are no duplicates

    //provide more even load of channels between controllers
    thread_rng().shuffle(&mut chans);
    //This could/should be Vec<Set>
    let chans_split: Vec<Vec<String>> = chans
        .chunks(CHANNELS_PER_CONTROLLER as usize)
        .map(|c| c.to_vec())
        .collect();
    let controllers: Vec<Controller> = chans_split
        .into_iter()
        .map(|s| Controller::init_with_sender(s, db_conn.clone()).unwrap())
        .collect();

    //NOTE: crashes, same issue as https://github.com/aatxe/irc/issues/174, possible solution is use more reactors with fewer channels each
    //TODO load balancing: have y total channels and x reactors. For each figure out to leave,
    //then append to join to get back to original capacity.

    loop {
        thread::sleep(Duration::from_secs(30));
        refresh_channels(&controllers);
    }
}

//TODO needs change to soak extra channels of the API doesn't behave at first but starts behaving
//itself later (ie top_channels.len() > sum of controllers.list().len())
fn refresh_channels(controllers: &[impl IrcController]) {
    refresh_channels_inner(controllers, channels::top_connections(MAX_CHANNELS))
}

//split out for testing purposes
fn refresh_channels_inner(controllers: &[impl IrcController], channels: Vec<String>) {
    let mut top_channels: HashSet<String> = HashSet::from_iter(channels.into_iter());

    let mut to_leave: Vec<Vec<_>> = (0..controllers.len()).map(|_| Vec::new()).collect();
    for (i, c) in controllers.iter().enumerate() {
        for l in c.list().unwrap() {
            if !top_channels.remove(&l) {
                to_leave[i].push(l);
            }
        }
    }

    let mut it = top_channels.drain(); //To join
    for (i, v) in to_leave.iter().enumerate() {
        for leaving in v {
            controllers[i].join(it.next().unwrap());
            controllers[i].part(leaving.to_string());
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use controller::test::*;

    #[test]
    fn basic_test() {
        let (_, controller) = Controller::init(Vec::new()).unwrap();
        let top_channel = channels::top_connections(1).get(0).unwrap().to_string();
        controller.join(top_channel.clone()).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1)); //TODO only added to joined list when message received from server
        assert_eq!(controller.list(), Some(vec![top_channel.clone()]));

        controller.part(top_channel.clone()).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1)); //TODO only added to joined list when message received from server
        assert_eq!(controller.list().unwrap(), Vec::<String>::new());
    }

    #[test]
    fn test_join_through_reference() {
        let controller = TestController::new();
        (&controller).join("a_channel".to_string());
        assert_eq!(controller.list(), Some(vec!["a_channel".to_string()]));
    }

    fn channel_list(controllers: &[impl IrcController]) -> Vec<String> {
        controllers
            .iter()
            .map(|c| c.list().unwrap())
            .flat_map(|v| v.into_iter())
            .collect()
    }

    //TODO might be nice to have a property test here
    fn assert_refresh_works(initial_channels: Vec<Vec<String>>, final_channels: Vec<Vec<String>>) {
        let controllers: Vec<_> = initial_channels
            .clone()
            .into_iter()
            .map(|v| TestController::init(v))
            .collect();
        refresh_channels_inner(&controllers, final_channels.concat());
        let refresh_channels: Vec<String> = channel_list(&controllers);

        let refresh_channels_set =
            HashSet::<String>::from_iter(refresh_channels.clone().into_iter());
        assert_eq!(refresh_channels.len(), refresh_channels_set.len());
        assert_eq!(
            HashSet::from_iter(
                final_channels
                    .clone()
                    .into_iter()
                    .flat_map(|v| v.into_iter())
            ),
            refresh_channels_set
        );
    }

    #[test]
    fn test_refresh_channels_no_op() {
        let new_channels: Vec<Vec<_>> = (0..10).map(|i| vec![i.to_string()]).collect();
        assert_refresh_works(new_channels.clone(), new_channels);
    }

    #[test]
    fn test_refresh_channels_single_replacement() {
        let channels: Vec<Vec<_>> = (0..100).map(|i| vec![i.to_string()]).collect();
        let mut new_channels = channels.clone();
        new_channels.pop();
        new_channels.push(vec!["101".to_string()]);
        assert_eq!(channels.len(), new_channels.len());
        assert_refresh_works(channels, new_channels);
    }

    #[test]
    fn test_channels_replace_all() {
        let channels: Vec<Vec<_>> = (0..100).map(|i| vec![i.to_string()]).collect();
        let new_channels: Vec<Vec<_>> = (300..400).map(|i| vec![i.to_string()]).collect();
        assert_eq!(channels.len(), new_channels.len());
        assert_refresh_works(channels, new_channels);
    }
}
