//TODO use failure crate?
#[derive(Debug)]
pub enum MyError {
    Db(Box<dyn std::error::Error>),
    Parse(&'static str),
    DotEnv(std::env::VarError),
    Other(Box<dyn std::error::Error>),
}

impl From<std::env::VarError> for MyError {
    fn from(e: std::env::VarError) -> Self {
        MyError::DotEnv(e)
    }
}

impl From<diesel::ConnectionError> for MyError {
    fn from(e: diesel::ConnectionError) -> Self {
        MyError::Db(Box::new(e))
    }
}
