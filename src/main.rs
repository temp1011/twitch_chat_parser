extern crate irc;

use irc::client::prelude::*;
use irc::proto::message::Tag;

fn main() {
    // We can also load the Config at runtime via Config::load("path/to/config.toml")
    let config = Config {
        nickname: Some("justinfan123123".to_owned()),
        server: Some("irc.chat.twitch.tv".to_owned()),
        channels: Some(vec!["#summit1g".to_owned()]),
        ..Config::default()
    };

    let mut reactor = IrcReactor::new().unwrap();
    let client = reactor.prepare_client_and_connect(&config).unwrap();
     client.send_cap_req(&[irc::proto::caps::Capability::Custom("twitch.tv/tags")]).unwrap(); 
    client.identify().unwrap();

    reactor.register_client_with_handler(client, |client, message| {
        match message.command {
            Command::PRIVMSG(ref target, ref msg) => {
                print!("{}, {}, {:?}\n\n\n", msg, target, format_tag(message.tags.unwrap()));
            },
            Command::PING(ref target, ref msg) => {
                client.send_pong(msg.clone().unwrap_or(String::from(""))).unwrap();
            },
            Command::JOIN(ref chan, _, _) => println!("joined {}", chan),
            _ => {} //dbg!(message.command)
        }
        Ok(())
    });

    reactor.run().unwrap();
}

//TODO - stream this instead? (map etc)
fn format_tag(tags: Vec<Tag>) -> Vec<String> {
    let mut ret: Vec<String> = Vec::with_capacity(tags.len());
    for t in tags {
        let mut s1 = t.0;
        let s2 = &t.1.unwrap();
        s1.push_str("=");
        s1.push_str(s2);
        ret.push(s1);
    }
    ret
}
