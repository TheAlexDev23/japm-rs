use std::{convert::From, fmt::Display};

use serde_json::Error as JsonError;

use crate::Package;

#[derive(Debug)]
pub enum Error {
    Json(JsonError),
}

impl From<JsonError> for Error {
    fn from(json_error: JsonError) -> Error {
        Error::Json(json_error)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Json(error) => write!(f, "Json Error: {}", error),
            // _ => write!(f, "Unkown error"),
        }
    }
}

impl Package {
    pub fn from_json(json: &String) -> Result<Package, Error> {
        let package: Package = serde_json::from_str(&json)?;

        Ok(package)
    }
}