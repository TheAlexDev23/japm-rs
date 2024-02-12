use crate::db::PackagesDb;
use crate::package::{LocalPackage, RemotePackage};

pub struct MockPackagesDb {
    installed_packges: Vec<LocalPackage>,
}

impl MockPackagesDb {
    pub fn new() -> MockPackagesDb {
        MockPackagesDb {
            installed_packges: Vec::new(),
        }
    }
}

impl PackagesDb for MockPackagesDb {
    fn add_package(&mut self, package: &RemotePackage) -> Result<(), String> {
        let local_packge = LocalPackage {
            package_data: package.package_data.clone(),
            remove: package.remove.clone(),
            dependencies: package.dependencies.clone(),
        };

        self.installed_packges.push(local_packge);

        Ok(())
    }

    fn remove_package(&mut self, package_name: &str) -> Result<(), String> {
        let index = self
            .installed_packges
            .iter()
            .position(|p| p.package_data.name == package_name);

        if let Some(index) = index {
            self.installed_packges.remove(index);
            Ok(())
        } else {
            Err(String::from("Package not found"))
        }
    }

    fn get_package(&mut self, package_name: &str) -> Result<LocalPackage, String> {
        let package = self
            .installed_packges
            .iter()
            .find(|p| p.package_data.name == package_name);

        if let Some(package) = package {
            Ok(package.clone())
        } else {
            Err(String::from("Package not found"))
        }
    }

    fn get_all_packages(&mut self) -> Result<Vec<LocalPackage>, String> {
        Ok(self.installed_packges.clone())
    }

    fn get_depending_packages(&mut self, package_name: &str) -> Result<Vec<LocalPackage>, String> {
        let all_packages = self.get_all_packages()?;
        let mut depending_packages: Vec<LocalPackage> = Vec::new();

        let package_name = String::from(package_name);

        for package in all_packages.into_iter() {
            if package.dependencies.contains(&package_name) {
                depending_packages.push(package);
            }
        }

        Ok(depending_packages)
    }
}
