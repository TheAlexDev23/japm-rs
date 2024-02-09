use serde::Deserialize;

pub mod searching;

#[derive(Default, Debug, Deserialize, Clone)]
pub struct Package {
    pub package_data: PackageData,

    pub dependencies: Vec<String>,

    pub install: Vec<String>,
    pub remove: Vec<String>,

    pub files: Vec<RemoteFile>,
}

#[derive(Default, Debug, Deserialize, Clone)]
pub struct PackageData {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Default, Debug, Deserialize, Clone)]
pub struct RemoteFile {
    pub url: String,
    pub target_path: String,
}

impl Package {
    pub fn from_json(json: &str) -> Result<Package, String> {
        match serde_json::from_str(json) {
            Ok(package) => Ok(package),
            Err(error) => Err(format!("Error parsing json:\n{error}")),
        }
    }
}
