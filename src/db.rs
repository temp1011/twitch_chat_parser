use crate::models::Message;
use crate::schema::messages;
use crate::types::TwitchMessage;
use arrayvec::ArrayVec;
use diesel::prelude::*;
use dotenv::dotenv;
use std::env;
use std::sync::mpsc;

//TwitchMessageODO - handle errors better in this module

const BATCH_SIZE: usize = 100;
//wrapper over db connection to batch insert messages and make code a bit cleaner
//TwitchMessageODO - a way to clean up all the lifetime and type parameters. And is static a problem?
pub struct DB {
    conn: SqliteConnection, //move this to more generic connection to make it easier to swap db
    queue: (mpsc::Sender<TwitchMessage>, mpsc::Receiver<TwitchMessage>),
    batch: ArrayVec<[TwitchMessage; BATCH_SIZE]>,
}

impl DB {
    fn new() -> Result<DB, Box<std::error::Error>> {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL")?;
        let conn = SqliteConnection::establish(&database_url)?;
        let ret = DB {
            conn,
            queue: mpsc::channel::<TwitchMessage>(),
            batch: ArrayVec::new(),
        };
        Ok(ret)
    }

    pub fn connection() -> Result<mpsc::Sender<TwitchMessage>, Box<std::error::Error>> {
        let mut datab: DB = DB::new()?;
        let sender = datab.queue.0.clone();
        std::thread::spawn(move || {
            datab.run();
        });
        Ok(sender)
    }
    //TODO - implement these
    //this should panic if things are very broken eg - database disappears
    fn run(&mut self) -> () {
        let mut nr = 0;
        while let Ok(v) = self.queue.1.recv() {
            match self.batch.try_push(v) {
                Err(r) => {
                    let res = self.insert();
                    if let Ok(num) = res {
                        nr += num;
                        println!("messages inserted: {}", nr);
                    }
                    debug_assert!(self.batch.len() == 0);
                    self.batch.push(r.element());
                }
                _ => {}
            }
        }
    }

    //or custom drop impl?
    pub fn flush(&mut self) {}

    pub fn insert(&mut self) -> QueryResult<usize> {
        let records: Vec<Message> = self
            .batch
            .drain(0..BATCH_SIZE) //ugly...
            .map(Message::from)
            .collect();

        diesel::insert_into(messages::table)
            .values(records)
            .execute(&self.conn)
    }
}

//TODO - is this correct?
impl Drop for DB {
    fn drop(&mut self) {
        self.flush();
    }
}
