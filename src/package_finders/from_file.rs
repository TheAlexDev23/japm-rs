use std::{fs, io};

use thiserror::Error;

use crate::commands::PackageFinder;
use crate::package::RemotePackage;

pub struct FromFilePackageFinder;
#[derive(Error, Debug)]
pub enum PackageFindError {
    #[error("An IO error has occured: {0}")]
    IO(#[from] io::Error),
    #[error("A json error has occured: {0}")]
    Json(#[from] serde_json::Error),
}

impl PackageFinder for FromFilePackageFinder {
    type Error = PackageFindError;
    fn find_package(&self, package_name: &str) -> Result<Option<RemotePackage>, Self::Error> {
        let mut path: String = String::from(package_name);
        if !path.ends_with(".json") {
            path.push_str(".json");
        }

        if fs::metadata(&path).is_err() {
            return Ok(None);
        }

        let json_content = fs::read_to_string(path)?;
        Ok(Some(RemotePackage::from_json(&json_content)?))
    }
}
