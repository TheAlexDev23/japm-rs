use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::Read;

use log::{debug, info, warn};

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
    search_cache: HashMap<String, RemotePackage>,
}
impl DefaultPackageFinder {
    pub fn new(from_file: bool, config: &Config) -> DefaultPackageFinder {
        DefaultPackageFinder {
            from_file,
            remotes: config.remotes.values().cloned().collect(),
            search_cache: HashMap::new(),
        }
    }
}
impl PackageFinder for DefaultPackageFinder {
    type Error = PackageFindError;
    fn find_package(&mut self, package_name: &str) -> Result<Option<RemotePackage>, Self::Error> {
        info!("Searching for package {package_name}");

        if let Some(remote_package) = self.search_cache.get(package_name) {
            debug!("Package search cache hit");
            return Ok(Some(remote_package.clone()));
        }

        let json_content = if self.from_file {
            find_from_file(package_name)?
        } else {
            find_from_remote(package_name, &self.remotes)?
        };

        match json_content {
            None => Ok(None),
            Some(json_content) => {
                let package = RemotePackage::from_json(&json_content)?;
                self.search_cache
                    .insert(String::from(package_name), package.clone());
                Ok(Some(package))
            }
        }
    }
}

fn find_from_file(package_name: &str) -> Result<Option<String>, io::Error> {
    if fs::metadata(package_name).is_err() {
        return Ok(None);
    }

    let json_content = fs::read_to_string(package_name)?;
    Ok(Some(json_content))
}

fn find_from_remote(package_name: &str, remotes: &[String]) -> Result<Option<String>, io::Error> {
    let mut remotes = remotes.iter();
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

    Ok(Some(json_content))
}
