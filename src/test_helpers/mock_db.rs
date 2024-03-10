use super::errors::StringError;
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
    type AddError = StringError;
    type RemoveError = StringError;
    type GetError = StringError;

    fn add_package(&mut self, package: &RemotePackage) -> Result<(), Self::AddError> {
        let local_packge = LocalPackage {
            package_data: package.package_data.clone(),
            pre_remove: package.pre_remove.clone(),
            post_remove: package.post_remove.clone(),
            package_files: package.package_files.clone(),
            dependencies: package.dependencies.clone(),
        };

        self.installed_packges.push(local_packge);

        Ok(())
    }

    fn remove_package(&mut self, package_name: &str) -> Result<(), Self::RemoveError> {
        let index = self
            .installed_packges
            .iter()
            .position(|p| p.package_data.name == package_name);

        if let Some(index) = index {
            self.installed_packges.remove(index);
            Ok(())
        } else {
            Err("Package not found".into())
        }
    }

    fn get_package(&mut self, package_name: &str) -> Result<Option<LocalPackage>, Self::GetError> {
        let package = self
            .installed_packges
            .iter()
            .find(|p| p.package_data.name == package_name);

        if let Some(package) = package {
            Ok(Some(package.clone()))
        } else {
            Ok(None)
        }
    }

    fn get_all_packages(&mut self) -> Result<Vec<LocalPackage>, Self::GetError> {
        Ok(self.installed_packges.clone())
    }

    fn get_depending_packages(
        &mut self,
        package_name: &str,
    ) -> Result<Vec<LocalPackage>, Self::GetError> {
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
