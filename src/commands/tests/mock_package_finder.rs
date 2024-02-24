use std::collections::HashMap;

use crate::package::{PackageData, RemotePackage};

use crate::commands::PackageFinder;

pub struct MockPackageFinder {
    packages_db: HashMap<String, RemotePackage>,
}

impl PackageFinder for MockPackageFinder {
    type Error = String;

    async fn find_package(&mut self, package_name: &str) -> Result<Option<RemotePackage>, String> {
        Ok(self.packages_db.get(&String::from(package_name)).cloned())
    }
}

impl MockPackageFinder {
    pub fn new() -> MockPackageFinder {
        let mut packages_db = HashMap::new();
        packages_db.insert(
            String::from("simple_package"),
            RemotePackage {
                package_data: PackageData {
                    name: String::from("simple_package"),
                    version: String::from("0.0.1"),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        packages_db.insert(
            String::from("package_with_dependency"),
            RemotePackage {
                package_data: PackageData {
                    name: String::from("package_with_dependency"),
                    version: String::from("0.0.1"),
                    ..Default::default()
                },
                dependencies: vec![String::from("simple_package")],
                ..Default::default()
            },
        );

        MockPackageFinder { packages_db }
    }

    pub fn update_remote_package_version(&mut self, package_name: &str) {
        self.packages_db
            .get_mut(package_name)
            .unwrap()
            .package_data
            .version = String::from("0.0.2");
    }

    pub async fn get_simple_packge(&mut self) -> RemotePackage {
        self.find_package("simple_package").await.unwrap().unwrap()
    }

    pub async fn get_package_with_dependency(&mut self) -> RemotePackage {
        self.find_package("package_with_dependency")
            .await
            .unwrap()
            .unwrap()
    }
}
