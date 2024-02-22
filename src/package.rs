use serde::Deserialize;

#[derive(Default, Debug, Deserialize, Clone, Hash, PartialEq, Eq)]
pub struct RemotePackage {
    pub package_data: PackageData,

    #[serde(default)]
    pub dependencies: Vec<String>,

    #[serde(default)]
    pub pre_install: Vec<String>,
    pub install: Vec<String>,
    #[serde(default)]
    pub post_install: Vec<String>,

    #[serde(default)]
    pub pre_remove: Vec<String>,
    /// Is empty until install action on package is performed
    #[serde(skip_deserializing)]
    pub package_files: Vec<String>,
    #[serde(default)]
    pub post_remove: Vec<String>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct LocalPackage {
    pub package_data: PackageData,

    pub dependencies: Vec<String>,

    pub pre_remove: Vec<String>,
    pub package_files: Vec<String>,
    pub post_remove: Vec<String>,
}

#[derive(Default, Debug, Deserialize, Clone, Hash, PartialEq, Eq)]
pub struct PackageData {
    pub name: String,
    pub version: String,
    pub description: String,
}

impl RemotePackage {
    pub fn from_json(json: &str) -> Result<RemotePackage, serde_json::Error> {
        serde_json::from_str(json)
    }
}
