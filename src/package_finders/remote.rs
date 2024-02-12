use std::io::Read;

use log::{debug, warn};

use reqwest::StatusCode;

use crate::{commands::PackageFinder, config::Config, package::RemotePackage};

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

impl PackageFinder for RemotePackageFinder {
    fn find_package(&self, package_name: &str) -> Result<crate::package::RemotePackage, String> {
        let mut remotes = self.remotes.iter();
        let json_content = loop {
            let mut remote = match remotes.next() {
                Some(remote) => remote.clone(),
                None => return Err(format!("Could not find package {package_name}")),
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
            if let Err(error) = res.read_to_string(&mut body) {
                return Err(format!("Error reading response body:\n{error}"));
            }

            break body;
        };

        RemotePackage::from_json(&json_content)
    }
}
