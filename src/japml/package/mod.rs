use serde::Deserialize;

pub mod parsing;

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

