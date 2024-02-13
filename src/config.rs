use serde_json::Value as JsonValue;

use std::{collections::HashMap, fs, path::Path};
use std::io;
use std::fmt::{self, Display};

use log::{info, trace};

#[cfg(test)]
mod tests;

pub struct Config {
    pub remotes: HashMap<String, String>,
}

const DEFAULT_CONFIG: &str = r#"
{
    "remotes": {
        "base": "https://raw.githubusercontent.com/TheAlexDev23/japm-official-packages/main/"
    }
}"#;

#[derive(Debug)]
pub enum Error {
    IO(io::Error),
    Json(serde_json::Error),
    Syntax(String),
}
impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Error::IO(error) => write!(f, "{error}"),
            Error::Json(error) => write!(f, "{error}"),
            Error::Syntax(error_message) => write!(f, "{error_message}"),
        }
    }
}
impl From<io::Error> for Error {
    fn from(other: io::Error) -> Self {
        Error::IO(other)
    }
}
impl From<serde_json::Error> for Error {
    fn from(other: serde_json::Error) -> Self {
        Error::Json(other)
    }
}


impl Config {
    pub fn create_default_config_if_necessary(config_path: &str) -> Result<(), io::Error> {
        trace!("Creating defalt configs if necessary");

        let config_path = Path::new(config_path);

        match config_path.try_exists()? {
            true => Ok(()),
            false => {
                info!("Config file does not exist. Creating new...");

                trace!("Creating config parent directories.");

                fs::create_dir_all(config_path.parent().unwrap())?;

                trace!("Creating and writing to config file.");

                fs::write(config_path, DEFAULT_CONFIG)?;

                Ok(())
            }
        }
    }

    pub fn from_file(config_path: &str) -> Result<Config, Error> {
        trace!("Parsing configs");

        let config_content = fs::read_to_string(config_path)?;

        Self::from_json(&config_content)
    }

    pub fn from_json(json_content: &str) -> Result<Config, Error> {
        Ok(Config {
            remotes: Self::get_remotes_from_config(json_content)?,
        })
    }

    fn get_remotes_from_config(config_content: &str) -> Result<HashMap<String, String>, Error> {
        trace!("Parsing config for remotes.");

        let root: JsonValue = serde_json::from_str(config_content)?;

        match root.get("remotes") {
            Some(remotes) => match remotes.as_object() {
                Some(remotes) => {
                    let mut return_map: HashMap<String, String> = HashMap::new();
                    for (key, value) in remotes.into_iter() {
                        if let JsonValue::String(url) = value {
                            return_map.insert(key.clone(), url.clone());
                        } else {
                            return Err(Error::Syntax(String::from(
                                "All keys and values in \"remotes\" should be strings",
                            )));
                        }
                    }

                    Ok(return_map)
                }
                None => Err(Error::Syntax(String::from("Remotes needs to be json object."))),
            },
            None => Err(Error::Syntax(String::from("Config has no remotes object."))),
        }
    }
}
