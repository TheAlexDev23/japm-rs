use std::fs;
use std::io::Read;

use reqwest::StatusCode;

use crate::logger::Logger;
use crate::Package;

#[derive(Clone)]
pub enum PackageSearchOptions<'a> {
    FromFile(&'a str),
    FromRemote { name: String, remotes: Vec<String> },
}

impl Package {
    pub fn find_package(options: PackageSearchOptions, logger: &Logger) -> Result<Package, String> {
        let content = match options {
            PackageSearchOptions::FromFile(path) => {
                let mut path: String = String::from(path);
                if !path.ends_with(".json") {
                    path.push_str(".json");
                }

                match fs::read_to_string(path) {
                    Ok(json_content) => json_content,
                    Err(error) => return Err(format!("Error reading package file:\n{error}")),
                }
            }
            PackageSearchOptions::FromRemote { name, remotes } => {
                let mut remotes = remotes.into_iter();
                loop {
                    let mut remote = match remotes.next() {
                        Some(remote) => remote,
                        None => return Err(format!("Could not find package {name}")),
                    };

                    remote.push_str(format!("/packages/{name}/package.json").as_str());

                    let mut res = match reqwest::blocking::get(remote) {
                        Ok(res) => {
                            if res.status() != StatusCode::OK {
                                logger.inf("Package {name} not found in remote {remote}");
                                continue;
                            }

                            res
                        }
                        Err(error) => {
                            logger.err(format!(
                                "Error while attempting to download package:\n{error}"
                            ));
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

        match Package::from_json(&content) {
            Ok(package) => Ok(package),
            Err(error) => return Err(format!("Error while parsing package:\n{error}")),
        }
    }
}
