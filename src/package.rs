use serde::Deserialize;

#[derive(Default, Debug, Deserialize, Clone, Hash, PartialEq, Eq)]
pub struct RemotePackage {
    pub package_data: PackageData,

    pub dependencies: Vec<String>,

    pub install: Vec<String>,
    pub remove: Vec<String>,

    pub files: Vec<RemoteFile>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct LocalPackage {
    pub package_data: PackageData,

    pub dependencies: Vec<String>,

    pub remove: Vec<String>,
}

#[derive(Default, Debug, Deserialize, Clone, Hash, PartialEq, Eq)]
pub struct PackageData {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Default, Debug, Deserialize, Clone, Hash, PartialEq, Eq)]
pub struct RemoteFile {
    pub url: String,
    pub target_path: String,
}

impl RemotePackage {
    pub fn from_json(json: &str) -> Result<RemotePackage, String> {
        match serde_json::from_str(json) {
            Ok(package) => Ok(package),
            Err(error) => Err(format!("Error parsing json:\n{error}")),
        }
    }
}
