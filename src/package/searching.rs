use std::fs;
use std::io::Read;

use log::{debug, warn};
use reqwest::StatusCode;

use crate::RemotePackage;

#[derive(Clone)]
pub enum PackageSearchOptions {
    FromFile,
    FromRemote(Vec<String>),
}

impl RemotePackage {
    pub fn find_package(
        name: &str,
        options: &PackageSearchOptions,
    ) -> Result<RemotePackage, String> {
        let content = match options {
            PackageSearchOptions::FromFile => {
                let mut path: String = String::from(name);
                if !path.ends_with(".json") {
                    path.push_str(".json");
                }

                match fs::read_to_string(path) {
                    Ok(json_content) => json_content,
                    Err(error) => return Err(format!("Error reading package file:\n{error}")),
                }
            }
            PackageSearchOptions::FromRemote(remotes) => {
                let mut remotes = remotes.iter();
                loop {
                    let mut remote = match remotes.next() {
                        Some(remote) => remote.clone(),
                        None => return Err(format!("Could not find package {name}")),
                    };

                    if remote.ends_with('/') {
                        remote.push_str(format!("/packages/{name}/package.json").as_str());
                    } else {
                        remote.push_str(format!("packages/{name}/package.json").as_str());
                    }

                    let mut res = match reqwest::blocking::get(&remote) {
                        Ok(res) => {
                            if res.status() != StatusCode::OK {
                                debug!("Package {name} not found in remote {remote}");
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
                    if let Err(error) = res.read_to_string(&mut body) {
                        return Err(format!("Error reading response body:\n{error}"));
                    }

                    break body;
                }
            }
        };

        match RemotePackage::from_json(&content) {
            Ok(package) => Ok(package),
            Err(error) => Err(format!("Error while parsing package:\n{error}")),
        }
    }
}
