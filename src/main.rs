extern crate irc;

use irc::client::prelude::*;
use irc::error::IrcError;
use irc::proto::message::Tag;

fn main() -> Result<(), IrcError> {
    let config = Config {
        nickname: Some("justinfan123123".to_owned()),
        server: Some("irc.chat.twitch.tv".to_owned()),
        channels: Some(vec!["#ninja".to_owned()]),
        ..Config::default()
    };

    let mut reactor = IrcReactor::new().unwrap();
    let client = setup_client(config, &mut reactor)?;

    reactor.register_client_with_handler(client, |client, message| {
        match message.command {
            Command::PRIVMSG(ref target, ref msg) => {
                let tags = match message.tags {
                    Some(t) => format_tags(t),
                    _ => Vec::with_capacity(0),
                };
                println!("{}, {}, {:?}", msg, target, tags);
            }
            Command::PING(_, msg) => {
                client.send_pong(msg.unwrap_or(String::from("")))?;
            }
            Command::JOIN(ref chan, _, _) => println!("joined {}", chan),
            _ => {} //dbg!(message.command)
        }
        Ok(())
    });

    reactor.run()
}

fn setup_client(config: Config, reactor: &mut IrcReactor) -> Result<IrcClient, IrcError> {
    let client = reactor.prepare_client_and_connect(&config)?;
    client.send_cap_req(&[irc::proto::caps::Capability::Custom("twitch.tv/tags")])?;
    client.identify()?;
    Ok(client)
}

//probably should be json or something fancy
fn format_tags(tags: Vec<Tag>) -> Vec<String> {
    tags.into_iter()
        .map(|t| {
            let mut s1 = t.0;
            let s2 = t.1.unwrap_or(String::from(""));
            s1.push_str("=");
            s1.push_str(&s2);
            s1
        })
        .collect()
}
