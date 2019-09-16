use irc::client::prelude::*;
use irc::error::IrcError;

use std::convert::TryFrom;
use std::io::{Error, ErrorKind};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use crate::types::TwitchMessage;

//TODO register many clients on the same reactor
pub struct Controller {
    client: IrcClient,
    sender: Sender<TwitchMessage>,
}

impl Controller {
    //optionally supply channels to init config with. Does this need result?
    pub fn init(channels: Vec<String>) -> Result<(Receiver<TwitchMessage>, Controller), IrcError> {
        run_client(channels)
    }

    //if you want to use a single receiver for many clients
    pub fn init_with_sender(
        channels: Vec<String>,
        sender: Sender<TwitchMessage>,
    ) -> Result<Controller, IrcError> {
        run_client_with_sender(channels, sender)
    }

    pub fn get_sender(&self) -> Sender<TwitchMessage> {
        self.sender.clone()
    }
}

impl IrcController for Controller {
    fn join(&self, channel: String) -> Result<(), IrcError> {
        self.client.send_join(channel)
    }

    //TODO maybe return option here
    fn list(&self) -> Option<Vec<String>> {
        self.client.list_channels()
    }

    fn part(&self, channel: String) -> Result<(), IrcError> {
        self.client.send_part(channel)
    }
}

pub trait IrcController {
    //note join also takes comma separated list in irc api
    fn join(&self, channel: String) -> Result<(), IrcError>;

    //will I need to store channels separately or is there a way in irc to get this?
    //update: storing means less latency but more problems if connection fails
    fn list(&self) -> Option<Vec<String>>;

    //return left channel. return error if not connected to this channel.
    fn part(&self, channel: String) -> Result<(), IrcError>;
}

//TODO need to return join handle?
fn run_client(chans: Vec<String>) -> Result<(Receiver<TwitchMessage>, Controller), IrcError> {
    let (send, recv) = channel::<TwitchMessage>();

    Ok((recv, run_client_inner(chans, send)))
}

fn run_client_with_sender(
    chans: Vec<String>,
    send: Sender<TwitchMessage>,
) -> Result<Controller, IrcError> {
    Ok(run_client_inner(chans, send))
}

fn run_client_inner(chans: Vec<String>, send: Sender<TwitchMessage>) -> Controller {
    let client = setup_client(chans).expect("Failed to setup client");
    let s = send.clone();

    let another_client = client.clone();
    thread::spawn(move || {
        let handler = move |client: &IrcClient, message: irc::proto::message::Message| {
            if let Ok(t_msg) = TwitchMessage::try_from(&message) {
                if let Err(e) = s.send(t_msg) {
                    Error::new(ErrorKind::Other, e);
                }
                return Ok(());
            }

            match message.command {
                Command::PING(_, msg) => {
                    client.send_pong(msg.unwrap_or_else(String::new))?;
                }
                Command::JOIN(ref chan, _, _) => println!("joined {}", chan),
                Command::PART(ref chan, _) => println!("left {}", chan),
                _ => {}
            }
            Ok(())
        };

        let mut reactor = IrcReactor::new().unwrap(); //TODO errors
        reactor.register_client_with_handler(another_client, handler);
        reactor.run();
    });

    let controller = Controller {
        client: client,
        sender: send.clone(),
    };

    controller
}

fn setup_client(chans: Vec<String>) -> Result<IrcClient, IrcError> {
    let mut nick = "justinfan".to_string();
    nick.push_str(&rand::random::<u32>().to_string());

    let config = Config {
        nickname: Some(nick),
        server: Some("irc.chat.twitch.tv".to_owned()),
        channels: Some(chans),
        ..Config::default()
    };
    let client = IrcClient::from_config(config)?;
    client.send_cap_req(&[irc::proto::caps::Capability::Custom("twitch.tv/tags")])?;
    client.identify()?;
    Ok(client)
}

#[cfg(test)]
pub mod test {
    use super::*;
    use std::collections::HashSet;
    use std::iter::FromIterator;
    use std::sync::Mutex;

    #[derive(Debug)]
    pub struct TestController {
        //the mutex here is a little awkward, but it's the best way I can think of to preserve the
        //trait signatures while storing the data locally
        chans: Mutex<HashSet<String>>,
    }

    impl IrcController for TestController {
        fn join(&self, channel: String) -> Result<(), IrcError> {
            self.chans.lock().unwrap().insert(channel);
            Ok(())
        }

        fn part(&self, channel: String) -> Result<(), IrcError> {
            self.chans.lock().unwrap().remove(&channel);
            Ok(())
        }

        fn list(&self) -> Option<Vec<String>> {
            Some(self.chans.lock().unwrap().clone().into_iter().collect())
        }
    }

    impl TestController {
        pub fn new() -> TestController {
            TestController {
                chans: Mutex::new(HashSet::new()),
            }
        }

        pub fn init(channels: Vec<String>) -> TestController {
            TestController {
                chans: Mutex::new(HashSet::from_iter(channels.into_iter())),
            }
        }
    }

}
