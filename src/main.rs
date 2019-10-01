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
    let mut chans = cleanup_channels(channels::top_connections(MAX_CHANNELS));

    let controllers = ControllerGroup::init_simple(chans, CHANNELS_PER_CONTROLLER, db_conn.clone());
    loop {
        thread::sleep(Duration::from_secs(30));
        refresh_channels(&controllers.controllers);
    }
}

//having to box here is unfortunate, but it's the only way to inject for testing that I could work
//out that the type system accepts
struct ControllerGroup {
    controllers: Vec<Box<IrcController>>,
    max_channels: u64,
    channels_per_controller: u64,
}

impl ControllerGroup {
    fn init(
        mut chans: Vec<String>,
        max_channels: u64,
        channels_per_controller: u64,
        conn: Sender<TwitchMessage>,
    ) -> ControllerGroup {
        Self::init_inner(
            chans,
            max_channels,
            channels_per_controller,
            conn,
            |s, conn| Box::new(Controller::init_with_sender(s, conn).unwrap()),
        )
    }

    fn init_inner(
        mut chans: Vec<String>,
        max_channels: u64,
        channels_per_controller: u64,
        conn: Sender<TwitchMessage>,
        constructor: fn(Vec<String>, Sender<TwitchMessage>) -> Box<IrcController>,
    ) -> ControllerGroup {
        thread_rng().shuffle(&mut chans);
        let chans_split: Vec<Vec<String>> = chans
            .chunks(channels_per_controller as usize)
            .map(|c| c.to_vec())
            .collect();
        let controllers: Vec<_> = chans_split
            .into_iter()
            .map(|s| constructor(s, conn.clone()))
            .collect();

        ControllerGroup {
            controllers,
            max_channels,
            channels_per_controller,
        }
    }

    fn init_simple(
        mut channels: Vec<String>,
        channels_per_controller: u64,
        conn: Sender<TwitchMessage>,
    ) -> ControllerGroup {
        let chans_length = channels.len() as u64;
        Self::init(channels, chans_length, channels_per_controller, conn)
    }

    fn list_channels(&self) -> Vec<Vec<String>> {
        self.controllers.iter().map(|c| c.list().unwrap()).collect()
    }
}

fn cleanup_channels(mut chans: Vec<String>) -> Vec<String> {
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

    chans
}

fn refresh_channels(controllers: &[Box<IrcController>]) {
    refresh_channels_inner(controllers, channels::top_connections(MAX_CHANNELS))
}

///split out for testing purposes
///There are 3 loops over all joined channels:
///1. Mark channels not returned in API to be left
///2. for the channels returned by the API, swap out a channel to be left for a fresh one
///3. If the API happens to return a higher (closer to expected) number of channels  than it did
///   last time then join these
///   too
fn refresh_channels_inner(controllers: &[Box<IrcController>], channels: Vec<String>) {
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
            //because of the API issues the length of the iterator can be shorter than the length
            //of the channels to leave
            match it.next() {
                Some(c) => {
                    controllers[i].join(c);
                    controllers[i].part(leaving.to_string());
                }
                None => break,
            }
        }
    }

    for (i, c) in controllers.iter().enumerate() {
        while c.list().unwrap().len() < CHANNELS_PER_CONTROLLER as usize {
            match it.next() {
                Some(ch) => {
                    controllers[i].join(ch);
                }
                None => break,
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use controller::test::*;

    //convenience
    impl ControllerGroup {
    
    fn init_test(
        mut channels: Vec<String>,
        channels_per_controller: u64,
    ) -> ControllerGroup {
        let chans_length = channels.len() as u64;
        Self::init_inner(channels, chans_length, channels_per_controller, channel().0, |c, _| Box::new(TestController::init(c)))
    }
    }

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
    fn assert_refresh_works(initial: ControllerGroup, final_channels: Vec<String>) {
        refresh_channels_inner(&initial.controllers, final_channels.clone());
        let refresh_channels: Vec<String> = initial.list_channels().into_iter().flatten().collect();

        let refresh_channels_set =
            HashSet::<String>::from_iter(refresh_channels.clone().into_iter());
        assert_eq!(refresh_channels.len(), refresh_channels_set.len());
        assert_eq!(
            HashSet::from_iter(
                final_channels
                    .clone()
                    .into_iter()
            ),
            refresh_channels_set
        );
    }

    #[test]
    fn test_refresh_channels_no_op() {
        let new_channels: Vec<_> = (0..10).map(|i| i.to_string()).collect();
        assert_refresh_works(ControllerGroup::init_test(new_channels.clone(), 2), new_channels);
    }

    #[test]
    fn test_refresh_channels_single_replacement() {
        let channels: Vec<_> = (0..100).map(|i| i.to_string()).collect();
        let mut new_channels = channels.clone();
        new_channels.pop();
        new_channels.push("101".to_string());
        assert_eq!(channels.len(), new_channels.len());
        assert_refresh_works(ControllerGroup::init_test(channels, 10), new_channels);
    }

    #[test]
    fn test_channels_replace_all() {
        let channels: Vec<_> = (0..100).map(|i| i.to_string()).collect();
        let new_channels: Vec<_> = (300..400).map(|i| i.to_string()).collect();
        assert_eq!(channels.len(), new_channels.len());
        assert_refresh_works(ControllerGroup::init_test(channels, 10), new_channels);
    }

    #[test]
    fn simulate_api_issues_refresh() {
        let channels: Vec<_> = (0..100).map(|i| i.to_string()).collect();
        let group = ControllerGroup::init_test(channels, 10);
        let new_channels: Vec<_> = (10..101).map(|i| i.to_string()).collect();
        refresh_channels_inner(&group.controllers, new_channels);
    }
}
