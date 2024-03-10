use std::error::Error;
use std::fmt::Display;

#[derive(Debug)]
pub struct StringError(String);

impl From<&str> for StringError {
    fn from(value: &str) -> Self {
        Self(String::from(value))
    }
}

impl Display for StringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for StringError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
    fn description(&self) -> &str {
        &self.0
    }
    fn cause(&self) -> Option<&dyn Error> {
        None
    }
}
