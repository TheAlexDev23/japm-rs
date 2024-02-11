use serde_json::Value as JsonValue;

use std::{collections::HashMap, fs, path::Path};

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

impl Config {
    pub fn create_default_config_if_necessary(config_path: &str) -> Result<(), String> {
        trace!("Creating defalt configs if necessary");

        let config_path = Path::new(config_path);

        match config_path.try_exists() {
            Ok(exists) => {
                if exists {
                    return Ok(());
                }

                trace!("Config file does not exist. Creating new...");

                trace!("Creating config parent directories.");

                if let Err(error) = fs::create_dir_all(config_path.parent().unwrap()) {
                    return Err(format!(
                        "Could not create remotes config directory recursively:\n{error}"
                    ));
                }

                trace!("Creating and writing to config file.");

                if let Err(error) = fs::write(config_path, DEFAULT_CONFIG) {
                    Err(format!("Could not write default remotes config:\n{error}"))
                } else {
                    Ok(())
                }
            }
            Err(error) => Err(format!(
                "Could not verify if {} exists:\n{error}",
                config_path.to_str().unwrap()
            )),
        }
    }

    pub fn from_file(config_path: &str) -> Result<Config, String> {
        trace!("Parsing configs");

        let config_content = match fs::read_to_string(config_path) {
            Ok(content) => content,
            Err(error) => return Err(format!("Error while reading remotes config\n:{error}")),
        };

        let config = Config {
            remotes: match Self::get_remotes_from_config(&config_content) {
                Ok(remotes) => remotes,
                Err(error) => return Err(format!("Could not get remotes:\n{error}")),
            },
        };

        Ok(config)
    }

    fn get_remotes_from_config(config_content: &str) -> Result<HashMap<String, String>, String> {
        trace!("Parsing config for remotes.");

        let root: JsonValue = match serde_json::from_str(config_content) {
            Ok(json_value) => json_value,
            Err(error) => return Err(format!("Error parsing remotes config:\n{error}")),
        };

        match root.get("remotes") {
            Some(remotes) => match remotes.as_object() {
                Some(remotes) => {
                    let mut return_map: HashMap<String, String> = HashMap::new();
                    for (key, value) in remotes.into_iter() {
                        if let JsonValue::String(url) = value {
                            return_map.insert(key.clone(), url.clone());
                        } else {
                            return Err(String::from(
                                "All keys and values in \"remotes\" should be strings",
                            ));
                        }
                    }

                    Ok(return_map)
                }
                None => Err(String::from("Could not get remotes as map.")),
            },
            None => Err(String::from("No remotes found in config")),
        }
    }
}
