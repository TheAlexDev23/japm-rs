use crate::package::{PackageData, RemotePackage};

use crate::commands::PackageFinder;

pub struct MockPackageFinder;
impl PackageFinder for MockPackageFinder {
    type Error = String;

    fn find_package(&self, package_name: &str) -> Result<Option<RemotePackage>, String> {
        Ok(match package_name {
            "simple_package" => Some(RemotePackage {
                package_data: PackageData {
                    name: String::from("simple_package"),
                    ..Default::default()
                },
                ..Default::default()
            }),
            "package_with_dependency" => Some(RemotePackage {
                package_data: PackageData {
                    name: String::from("package_with_dependency"),
                    ..Default::default()
                },
                dependencies: vec![String::from("simple_package")],
                ..Default::default()
            }),
            _ => None,
        })
    }
}

impl MockPackageFinder {
    pub fn get_simple_packge(&self) -> RemotePackage {
        self.find_package("simple_package").unwrap().unwrap()
    }

    pub fn get_package_with_dependency(&self) -> RemotePackage {
        self.find_package("package_with_dependency")
            .unwrap()
            .unwrap()
    }
}
