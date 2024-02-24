use std::collections::HashMap;
use std::io;
use std::path::Path;

use tokio::fs;

use serde_json::Value as JsonValue;

use thiserror::Error;

use log::trace;

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

#[derive(Error, Debug)]
pub enum Error {
    #[error("An IO error has occured: {0}")]
    IO(#[from] io::Error),
    #[error("A json parsing error has occured: {0}")]
    Json(#[from] serde_json::Error),
    #[error("The config file has invalid json syntax: {0}")]
    Syntax(String),
}

impl Config {
    pub async fn create_default_config_if_necessary(config_path: &str) -> Result<bool, io::Error> {
        trace!("Creating default configs if necessary");

        let config_path = Path::new(config_path);

        match config_path.try_exists()? {
            true => Ok(false),
            false => {
                trace!("Creating config file parent directories.");
                tokio::fs::create_dir_all(config_path.parent().unwrap()).await?;

                trace!("Creating config file.");
                fs::File::create(config_path).await?;

                Ok(true)
            }
        }
    }

    pub async fn write_default_config(config_path: &str) -> Result<(), io::Error> {
        fs::write(config_path, DEFAULT_CONFIG).await
    }

    pub async fn from_file(config_path: &str) -> Result<Config, Error> {
        trace!("Parsing configs");

        let config_content = fs::read_to_string(config_path).await?;

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
                None => Err(Error::Syntax(String::from(
                    "Remotes needs to be json object.",
                ))),
            },
            None => Err(Error::Syntax(String::from("Config has no remotes object."))),
        }
    }
}
