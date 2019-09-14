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

type Res = Result<Option<Vec<String>>, IrcError>;

pub struct Controller {
    //receive results of queries from the reactor
    irc_recv: Receiver<Res>, //TODO needs type of query result or whatever
    query_sender: UnboundedSender<Operation>, //or something, maybe needs the enum
    sender: Sender<TwitchMessage>,
}

impl Controller {
    //optionally supply channels to init config with. Does this need result?
    pub fn init(
        channels: Vec<String>,
    ) -> Result<(Receiver<TwitchMessage>, Controller), IrcError> {
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
    fn execute(&self, op: Operation) -> Res {
        self.query_sender.send(op);
        //reactor executes op here
        self.irc_recv.recv_timeout(Duration::from_secs(10)).unwrap() //TODO timeout value, errors
    }
}

pub trait IrcController {
    //note join also takes comma separated list
    fn join(&self, channel: String) -> Result<(), IrcError> {
        self.execute(Operation::Join(channel)).map(|_| ())
    }

    //will I need to store channels separately or is there a way in irc to get this?
    fn list(&self) -> Vec<String> {
        self.execute(Operation::List)
            .and_then(|o| Ok(o.unwrap_or_else(Vec::new)))
            .unwrap_or_default()
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

    let controller = Controller {
        irc_recv: res_receiver,
        query_sender: tx,
        sender: send.clone(),
    };

    thread::spawn(move || {
        let mut reactor = IrcReactor::new().unwrap(); //TODO errors
        let client = setup_client(&mut reactor, chans).expect("Failed to setup client"); //TODO errors
                                                                                         //TODO - use multiple clients for better parallelism, given that twitch seems to rate limit
                                                                                         //joining channels.
        reactor.register_client_with_handler(client.clone(), move |client, message| {
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
                Command::PART(ref chan, _) => println!("left {}", chan),
                _ => {}
            }
            Ok(())
        });

        let mut controller = ClientController { client };
        reactor.register_future(
            rx.for_each(move |op| {
                res_sender
                    .send(controller.handle_operation(op))
                    .map_err(|e| IrcError::NoUsableNick); //and then can receive result here (Hopefully with timeout and without too much trickery). TODO errors
                Ok(())
            })
            .map_err(|error| IrcError::NoUsableNick),
        ); //Custom{inner: error,})); TODO errors

        reactor.run();
    });
    controller
}

fn setup_client(reactor: &mut IrcReactor, chans: Vec<String>) -> Result<IrcClient, IrcError> {
    let mut nick = "justinfan".to_string();
    nick.push_str(&rand::random::<u32>().to_string());

    let config = Config {
        nickname: Some(nick),
        server: Some("irc.chat.twitch.tv".to_owned()),
        channels: Some(chans),
        ..Config::default()
    };
    let client = reactor.prepare_client_and_connect(&config)?;
    client.send_cap_req(&[irc::proto::caps::Capability::Custom("twitch.tv/tags")])?;
    client.identify()?;
    Ok(client)
}
