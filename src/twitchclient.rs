use crate::types::TwitchMessage;
use std::convert::TryFrom;
use std::sync::mpsc;
use std::sync::Arc;
use tokio::stream::StreamExt as _;
use twitchchat::{
    events, messages, rate_limit::RateClass, Dispatcher, IntoChannel, RateLimit, Runner, Status,
};
use twitchchat::{Capability, EventStream, UserConfig};

async fn setup(dispatcher: Dispatcher) -> EventStream<Arc<messages::Privmsg<'static>>> {
    let events = dispatcher.subscribe::<events::Privmsg>();

    let ready = dispatcher.wait_for::<events::IrcReady>().await.unwrap();
    eprintln!("joined with nick {}", ready.nickname);
    events
}

async fn run(dispatcher: Dispatcher, sender: mpsc::Sender<TwitchMessage>) {
    let mut events = setup(dispatcher).await;

    while let Some(msg) = events.next().await {
        sender.send(TwitchMessage::try_from(msg).unwrap()).unwrap();
    }
}

pub async fn get_messages(
    channels: Vec<impl IntoChannel + std::fmt::Display + std::clone::Clone + Send + Sync + 'static>,
    sender: mpsc::Sender<TwitchMessage>,
) {
    let dispatcher = Dispatcher::new();
    let (runner, mut control) =
        Runner::new(dispatcher.clone(), RateLimit::from_class(RateClass::Known));
    let mut writer = control.writer().clone();

    let user_config = UserConfig::builder()
        .anonymous()
        .capabilities(&[Capability::Tags])
        .build()
        .unwrap();
    // connect to twitch
    let conn = twitchchat::connect_tls(&user_config).await.unwrap();
    // and run the dispatcher/writer loop
    let done = runner.run(conn);

    //    let printer = tokio::spawn(async move {
    //     let mut counter = 0u64;
    //     while let Some(msg) = comm_recv.next().await {
    //         counter += 1;
    // //           if (counter % 1) == 0 {
    // //               eprintln!("received {} messages", counter);
    // //           }
    // //           TODO parsing here is probably awful for performance?
    //         println!("received a {}th message {:?}",counter, TwitchMessage::try_from(msg));

    //     }
    // });

    let message_receiver = tokio::spawn(async move {
        run(dispatcher, sender).await;
    });

    let joiner = tokio::spawn(async move {
        eprintln!("joining channels");
        //TODO need to try and join channels concurrently
        for c in channels {
            writer.join(c.clone()).await.unwrap();
            eprintln!("joined {}", c);
        }
        eprintln!("done joining channels");
    });
    tokio::select! {
        _ = joiner => { eprintln!("joiner task crashed") }
        // wait for the bot to complete
        _ = message_receiver => { eprintln!("done running the bot") }
        // or wait for the runner to complete
        status = done => {
            match status {
                Ok(Status::Canceled) => { eprintln!("runner was canceled") }
                Ok(Status::Eof) => { eprintln!("got an eof, exiting") }
                Ok(Status::Timeout) => { eprintln!("client connection timed out") }
                Err(err) => { eprintln!("error running: {}", err) }
            }
        }
    }
}
