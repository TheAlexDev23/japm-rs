use std::io;
use std::io::Read;

use thiserror::Error;

use log::{debug, warn};

use reqwest::StatusCode;

use crate::package::RemotePackage;
use crate::{commands::PackageFinder, config::Config};

#[derive(Debug)]
pub struct RemotePackageFinder {
    remotes: Vec<String>,
}

impl RemotePackageFinder {
    pub fn new(config: &Config) -> RemotePackageFinder {
        RemotePackageFinder {
            remotes: config.remotes.values().cloned().collect(),
        }
    }
}

#[derive(Error, Debug)]
pub enum PackageFindError {
    #[error("Could not read response body: {0}")]
    Read(#[from] io::Error),
    #[error("A json error has occured: {0}")]
    Json(#[from] serde_json::Error),
}

impl PackageFinder for RemotePackageFinder {
    type Error = PackageFindError;
    fn find_package(&self, package_name: &str) -> Result<Option<RemotePackage>, Self::Error> {
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

        Ok(Some(RemotePackage::from_json(&json_content)?))
    }
}
