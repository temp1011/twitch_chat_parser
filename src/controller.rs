use futures::sync::mpsc;
use futures::sync::mpsc::UnboundedReceiver;
use irc::client::prelude::*;
use irc::error::IrcError;

use futures::sync::mpsc::UnboundedSender;
use std::convert::TryFrom;
use std::io::{Error, ErrorKind};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;

use crate::types::TwitchMessage;
use std::sync::{Arc, Mutex, MutexGuard};

type Res = Result<Option<Vec<String>>, IrcError>;

pub struct Controller {
    client: Arc<Mutex<IrcClient>>,
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
        println!("joining, {}", channel);
        let guard = self.client.lock().unwrap();
        (*guard).send_join(channel)
    }

    //TODO maybe return option here
    fn list(&self) -> Option<Vec<String>> {
        let guard = self.client.lock().unwrap();
        (*guard).list_channels()
    }

    fn part(&self, channel: String) -> Result<(), IrcError> {
        let guard = self.client.lock().unwrap();
        (*guard).send_part(channel)
    }

    fn execute(&self, op: Operation) -> Res {
        Ok(None) //TODO
    }
}

pub trait IrcController {
    //note join also takes comma separated list
    fn join(&self, channel: String) -> Result<(), IrcError> {
        self.execute(Operation::Join(channel)).map(|_| ())
    }

    //will I need to store channels separately or is there a way in irc to get this?
    fn list(&self) -> Option<Vec<String>> {
        self.execute(Operation::List).unwrap()
    }

    //return left channel. return error if not connected to this channel.
    fn part(&self, channel: String) -> Result<(), IrcError> {
        self.execute(Operation::Part(channel)).map(|_| ())
    }

    fn execute(&self, op: Operation) -> Res;
}

enum Operation {
    Join(String),
    Part(String),
    List,
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
    let (tx, rx) = mpsc::unbounded::<Operation>();
    let (res_sender, res_receiver) = channel::<Res>();

    let client = Arc::new(Mutex::new(
        setup_client(chans).expect("Failed to setup client"),
    )); //TODO errors and see docs: https://docs.rs/irc/0.13.6/irc/client/struct.IrcClient.html#method.from_config could panic a lot
    let s = send.clone();

    let c = Arc::clone(&client);
    let d = Arc::clone(&client);
    thread::spawn(move || {
        //TODO - use multiple clients for better parallelism, given that twitch seems to rate limit
        //joining channels.

        let mut handler = move |c: Arc<Mutex<IrcClient>>, message: irc::proto::message::Message| {
            let guard = c.lock().unwrap();
            if let Ok(t_msg) = TwitchMessage::try_from(&message) {
                if let Err(e) = s.send(t_msg) {
                    Error::new(ErrorKind::Other, e);
                }
                return Ok(());
            }

            match message.command {
                Command::PING(_, msg) => {
                    (*guard).send_pong(msg.unwrap_or_else(String::new))?;
                }
                Command::JOIN(ref chan, _, _) => println!("joined {}", chan),
                Command::PART(ref chan, _) => println!("left {}", chan),
                _ => {}
            }
            Ok(())
        };

        let mut reactor = IrcReactor::new().unwrap(); //TODO errors
        let guard: MutexGuard<IrcClient> = c.lock().unwrap();
        reactor.register_future((*guard).stream().for_each(move |message| {
            let e = Arc::clone(&d);
        handler(e, message)}));
        reactor.run();
    });

    let controller = Controller {
        client: Arc::clone(&client),
        sender: send.clone(),
    };

    controller
}

struct ClientController {
    client: IrcClient,
}

impl ClientController {
    fn handle_operation(&mut self, op: Operation) -> Res {
        match op {
            Operation::Join(chan) => {
                println!("joined {}", chan);
                self.client.send_join(chan).map(|_| None)
            }
            Operation::Part(chan) => {
                println!("left {}", chan);
                self.client.send_part(chan).map(|_| None)
            }
            Operation::List => Ok(self.client.list_channels()),
        }
    }
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
    use std::sync::Mutex;

    pub struct TestController {
        //the mutex here is a little awkward, but it's the best way I can think of to preserve the
        //trait signatures while storing the data locally
        chans: Mutex<HashSet<String>>,
    }

    impl IrcController for TestController {
        fn execute(&self, op: Operation) -> Res {
            let mut cs = self.chans.lock().unwrap();
            match op {
                Operation::Join(c) => {
                    cs.insert(c);
                    Ok(None)
                }
                Operation::Part(c) => {
                    cs.remove(&c);
                    Ok(None)
                }
                Operation::List => Ok(Some(cs.clone().into_iter().collect())),
            }
        }
    }

    impl TestController {
        pub fn new() -> TestController {
            TestController {
                chans: Mutex::new(HashSet::new()),
            }
        }
    }

}
