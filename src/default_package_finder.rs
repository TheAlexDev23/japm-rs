use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::Read;

use log::{debug, trace, warn};

use reqwest::StatusCode;

use thiserror::Error;

use crate::commands::PackageFinder;
use crate::config::Config;
use crate::package::RemotePackage;

#[derive(Error, Debug)]
pub enum PackageFindError {
    #[error("An io error has occured: {0}")]
    Read(#[from] io::Error),
    #[error("A json error has occured: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct DefaultPackageFinder {
    from_file: bool,
    remotes: Vec<String>,
    remote_search_cache: HashMap<String, RemotePackage>,
}
impl DefaultPackageFinder {
    pub fn new(from_file: bool, config: &Config) -> DefaultPackageFinder {
        DefaultPackageFinder {
            from_file,
            remotes: config.remotes.values().cloned().collect(),
            remote_search_cache: HashMap::new(),
        }
    }
}
impl PackageFinder for DefaultPackageFinder {
    type Error = PackageFindError;
    fn find_package(&mut self, package_name: &str) -> Result<Option<RemotePackage>, Self::Error> {
        if self.from_file {
            let mut path: String = String::from(package_name);
            if !path.ends_with(".json") {
                path.push_str(".json");
            }

            if fs::metadata(&path).is_err() {
                return Ok(None);
            }

            let json_content = fs::read_to_string(path)?;
            Ok(Some(RemotePackage::from_json(&json_content)?))
        } else if let Some(remote_package) =
            self.remote_search_cache.get(&String::from(package_name))
        {
            trace!("Remote package chache hit for {package_name}");
            Ok(Some(remote_package.clone()))
        } else {
            let mut remotes = self.remotes.iter();
            let json_content = loop {
                let mut remote = match remotes.next() {
                    Some(remote) => remote.clone(),
                    None => return Ok(None),
                };

                if remote.ends_with('/') {
                    remote.push_str(format!("/packages/{package_name}/package.json").as_str());
                } else {
                    remote.push_str(format!("packages/{package_name}/package.json").as_str());
                }

                let mut res = match reqwest::blocking::get(&remote) {
                    Ok(res) => {
                        if res.status() != StatusCode::OK {
                            debug!("Package {package_name} not found in remote {remote}");
                            continue;
                        }

                        res
                    }
                    Err(error) => {
                        warn!("Error while attempting to download package:\n{error}");
                        continue;
                    }
                };

                let mut body = String::new();
                res.read_to_string(&mut body)?;
                break body;
            };

            let remote_package = RemotePackage::from_json(&json_content)?;

            self.remote_search_cache
                .insert(String::from(package_name), remote_package.clone());

            Ok(Some(remote_package))
        }
    }
}
