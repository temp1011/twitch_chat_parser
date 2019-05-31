
#[derive(Debug)]
pub enum MyError {
    Irc(irc::error::IrcError),
    Db(Box<std::error::Error>),
    Parse(Box<std::error::Error>),
    Other(Box<std::error::Error>),
}

impl From<irc::error::IrcError> for MyError {
    fn from(e: irc::error::IrcError) -> Self {
        MyError::Irc(e)
    }
}
