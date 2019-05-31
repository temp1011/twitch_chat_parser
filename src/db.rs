use crate::models::Message;
use crate::schema::messages;
use crate::types::TwitchMessage;
use arrayvec::ArrayVec;
use diesel::prelude::*;
use dotenv::dotenv;
use std::env;
use std::sync::mpsc;

//TODO - handle errors better in this module

const BATCH_SIZE: usize = 1024;
//wrapper over db connection to batch insert messages and make code a bit cleaner. Also allows
//easier use of database while program is running since batching means the db isn't constantly
//locked.
//TODO - try to make this less database dependant
pub struct DB {
    conn: SqliteConnection,
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
    //TODO - this should panic if things are very broken eg - database disappears
    fn run(&mut self) {
        let mut nr = 0;
        while let Ok(v) = self.queue.1.recv() {
            if let Err(returned) = self.batch.try_push(v) {
                if let Ok(num) = self.flush() {
                    nr += num;
                    println!("messages inserted: {}", nr);
                }
                self.batch.push(returned.element());
            }
        }
    }

    //without assert can just be inlined or potentially some error handling. I just need tests I
    //think.
    pub fn flush(&mut self) -> Result<usize, diesel::result::Error> {
        match self.insert() {
            Ok(num) => {
                debug_assert!(self.batch.len() == 0);
                Ok(num)
            }
            Err(e) => Err(e),
        }
    }

    fn insert(&mut self) -> QueryResult<usize> {
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
        self.flush(); //look at result here?
    }
}
