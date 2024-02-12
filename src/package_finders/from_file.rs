use std::fs;

use crate::commands::PackageFinder;
use crate::package::RemotePackage;

pub struct FromFilePackageFinder;

impl PackageFinder for FromFilePackageFinder {
    fn find_package(&self, package_name: &str) -> Result<RemotePackage, String> {
        let mut path: String = String::from(package_name);
        if !path.ends_with(".json") {
            path.push_str(".json");
        }

        match fs::read_to_string(path) {
            Ok(json_content) => Ok(RemotePackage::from_json(&json_content)?),
            Err(error) => Err(format!("Error reading package file:\n{error}")),
        }
    }
}
