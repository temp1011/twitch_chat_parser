use crate::error::MyError;
use crate::models::Message;
use crate::schema::messages;
use crate::types::TwitchMessage;
use diesel::prelude::*;
use dotenv::dotenv;
use std::env;
use std::sync::mpsc;
use chrono::Utc;
//TODO - handle errors better in this module

const BATCH_SIZE: usize = 1024;
//wrapper over db connection to batch insert messages and make code a bit cleaner. Also allows
//easier use of database while program is running since batching means the db isn't constantly
//locked.
//TODO - try to make this less database backend dependant
pub struct DB {
    conn: SqliteConnection,
    queue: (mpsc::Sender<TwitchMessage>, mpsc::Receiver<TwitchMessage>),
    //this used to use ArrayVec but there was an issue with stackoverflow on debug builds
    batch: Vec<TwitchMessage>,
}

impl DB {
    fn new() -> Result<DB, MyError> {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL")?;
        let conn = SqliteConnection::establish(&database_url)?;
        let ret = DB {
            conn,
            queue: mpsc::channel::<TwitchMessage>(),
            batch: Vec::with_capacity(BATCH_SIZE),
        };
        Ok(ret)
    }

    //should this be a method on the db instead and multiple calls just clones the sender?
    pub fn connection() -> Result<mpsc::Sender<TwitchMessage>, MyError> {
        let mut datab: DB = DB::new()?;
        let sender = datab.queue.0.clone();
        std::thread::spawn(move || {
            datab.run();
        });
        Ok(sender)
    }
    //TODO - this should panic if things are very broken eg - database disappears
    fn run(&mut self) {
        let mut nr = 0;
        while let Ok(v) = self.queue.1.recv() {
            self.batch.push(v);
            if self.batch.len() >= BATCH_SIZE {
                if let Ok(num) = self.flush() {
                    nr += num;
                    println!("[{}] messages inserted: {}", Utc::now(), nr);
                }
            }
        }
    }

    //without assert can just be inlined or potentially some error handling. I just need tests I
    //think.
    pub fn flush(&mut self) -> Result<usize, MyError> {
        match self.insert() {
            Ok(num) => {
                debug_assert!(self.batch.is_empty());
                Ok(num)
            }
            Err(e) => Err(MyError::Db(Box::new(e))),
        }
    }

    fn insert(&mut self) -> QueryResult<usize> {
        let records: Vec<Message> = self.batch.drain(..).map(Message::from).collect();

        diesel::insert_into(messages::table)
            .values(records)
            .execute(&self.conn)
    }
}

//TODO - is this correct?
impl Drop for DB {
    fn drop(&mut self) {
        self.flush(); //look at result here?
    }
}
