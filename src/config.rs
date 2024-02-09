use serde_json::Value as JsonValue;

use std::{collections::HashMap, fs, path::Path};

pub struct Config {
    pub remotes: HashMap<String, String>,
}

const REMOTES_CONFIG_PATH: &str = "/etc/japm/remotes.json";

impl Config {
    pub fn create_default_config_if_doesnt_exist() -> Result<(), String> {
        Self::verify_if_remotes_exists_or_create_default()
    }

    pub fn from_default_config() -> Result<Config, String> {
        let config = Config {
            remotes: match Self::get_remotes_from_config() {
                Ok(remotes) => remotes,
                Err(error) => return Err(format!("Could not get remotes:\n{error}")),
            },
        };

        Ok(config)
    }

    fn verify_if_remotes_exists_or_create_default() -> Result<(), String> {
        let remotes_config = Path::new(REMOTES_CONFIG_PATH);

        const DEFAULT_CONFIG: &str = r#"
        {
            "remotes": {
                "base": "https://raw.githubusercontent.com/TheAlexDev23/japm-official-packages/main/"
            }
        }
        "#;

        match remotes_config.try_exists() {
            Ok(exists) => {
                if exists {
                    return Ok(());
                }

                // .parent() won't fail as the hardcoded struct clearly has parent so .unwrap() is
                // ok
                if let Err(error) = fs::create_dir_all(remotes_config.parent().unwrap()) {
                    return Err(format!(
                        "Could not create remotes config directory recursively:\n{error}"
                    ));
                }

                if let Err(error) = fs::write(REMOTES_CONFIG_PATH, DEFAULT_CONFIG) {
                    Err(format!("Could not write default remotes config:\n{error}"))
                } else {
                    Ok(())
                }
            }
            Err(error) => Err(format!(
                "Could not verify if {REMOTES_CONFIG_PATH} exists:\n{error}"
            )),
        }
    }

    fn get_remotes_from_config() -> Result<HashMap<String, String>, String> {
        let remotes_content = match fs::read_to_string(REMOTES_CONFIG_PATH) {
            Ok(content) => content,
            Err(error) => return Err(format!("Error while reading remotes config\n:{error}")),
        };

        let root: JsonValue = match serde_json::from_str(&remotes_content) {
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
