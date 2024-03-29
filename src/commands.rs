use std::error::Error;

use async_recursion::async_recursion;

use log::{debug, info, trace};

use linked_hash_map::LinkedHashMap;
use semver::Version;

use crate::action::Action;
use crate::db::PackagesDb;
use crate::package::{LocalPackage, RemotePackage};
use crate::package_finder::PackageFinder;
use crate::progress::{self, ProgressType};

pub use errors::*;

/// A linked hash set ensures all items are present once and allows to iterate
/// in the same order items where inserted.
type LinkedHashSet<T> = LinkedHashMap<T, ()>;

pub mod errors;
#[cfg(test)]
mod tests;

pub enum ReinstallOptions {
    Update,
    ForceReinstall,
    Ignore,
}

pub async fn install_packages<EFind: Error, EDatabase: Error>(
    packages: Vec<String>,
    package_finder: &mut impl PackageFinder<Error = EFind>,
    reinstall_options: &ReinstallOptions,
    db: &mut impl PackagesDb<GetError = EDatabase>,
) -> Result<Vec<Action>, InstallError<EDatabase, EFind>> {
    let mut actions: LinkedHashSet<Action> = LinkedHashSet::new();

    progress::increment_target(ProgressType::Packages, packages.len() as i32).await;

    for package_name in packages.iter() {
        actions.extend(install_package(package_name, package_finder, reinstall_options, db).await?);

        progress::increment_completed(ProgressType::Packages, 1).await;
    }

    Ok(actions.keys().cloned().collect())
}

pub async fn remove_packages<EDatabase: Error>(
    package_names: Vec<String>,
    recursive: bool,
    db: &mut impl PackagesDb<GetError = EDatabase>,
) -> Result<Vec<Action>, RemoveError<EDatabase>> {
    let mut actions: LinkedHashSet<Action> = LinkedHashSet::new();

    progress::increment_target(ProgressType::Packages, package_names.len() as i32).await;

    for package_name in package_names.into_iter() {
        actions.extend(remove_package(&package_name, recursive, db).await?);
        progress::increment_completed(ProgressType::Packages, 1).await;
    }

    Ok(actions.keys().cloned().collect())
}

pub async fn update_all_packages<EDatabase: Error, EFind: Error>(
    package_finder: &mut impl PackageFinder<Error = EFind>,
    db: &mut impl PackagesDb<GetError = EDatabase>,
) -> Result<Vec<Action>, UpdateError<EDatabase, EFind>> {
    let packages = match db.get_all_packages() {
        Ok(packages) => packages,
        Err(error) => return Err(UpdateError::DatabaseGet(error)),
    };

    let packages = packages.into_iter().map(|p| p.package_data.name).collect();

    let actions = install_packages(packages, package_finder, &ReinstallOptions::Update, db).await?;

    Ok(actions)
}

pub async fn update_packages<EDatabase: Error, EFind: Error>(
    package_names: Vec<String>,
    package_finder: &mut impl PackageFinder<Error = EFind>,
    db: &mut impl PackagesDb<GetError = EDatabase>,
) -> Result<Vec<Action>, UpdateError<EDatabase, EFind>> {
    let mut actions: Vec<Action> = Vec::new();
    for package_name in package_names.into_iter() {
        let depending = match get_depending(&package_name, db, -1) {
            Ok(depending) => depending,
            Err(error) => return Err(UpdateError::DatabaseGet(error)),
        };

        let mut packages_to_update: Vec<String> =
            depending.into_iter().map(|p| p.package_data.name).collect();

        packages_to_update.push(package_name);

        actions.extend(
            install_packages(
                packages_to_update,
                package_finder,
                &ReinstallOptions::Update,
                db,
            )
            .await?,
        );
    }

    Ok(actions)
}

pub fn print_package_info<EDatabase: Error>(
    package_names: Vec<String>,
    db: &mut impl PackagesDb<GetError = EDatabase>,
) -> Result<(), InfoError<EDatabase>> {
    for package_name in package_names.into_iter() {
        let package = db.get_package(&package_name)?;
        if package.is_none() {
            return Err(InfoError::PackageNotInstalled(package_name));
        }

        let package = package.unwrap();

        info!(
            "Package {package_name}:
    version: {}
    description: {}
    dependencies: {:?}",
            package.package_data.version, package.package_data.description, package.dependencies
        );
    }

    Ok(())
}

#[async_recursion(?Send)]
async fn install_package<EFind: Error, EDatabase: Error>(
    package_name: &str,
    package_finder: &mut impl PackageFinder<Error = EFind>,
    reinstall_options: &ReinstallOptions,
    db: &mut impl PackagesDb<GetError = EDatabase>,
) -> Result<LinkedHashSet<Action>, InstallError<EDatabase, EFind>> {
    debug!("Generating install actions for package: {package_name}");

    let mut actions: LinkedHashSet<Action> = LinkedHashSet::new();

    let remote_package = match package_finder.find_package(package_name).await {
        Ok(package) => {
            if package.is_none() {
                return Err(InstallError::PackageNotFound(String::from(package_name)));
            }

            package.unwrap()
        }
        Err(error) => return Err(InstallError::Find(error)),
    };

    trace!("Found remote package:\n{remote_package:#?}");

    match db.get_package(&remote_package.package_data.name) {
        Ok(local_package) => {
            if let Some(local_package) = local_package {
                match reinstall_options {
                    ReinstallOptions::ForceReinstall => {
                        info!("Package {package_name} already installed, reinstalling...");
                        // It's also possible to call remove_package and get the package removal specific actions.
                        // But this can cause issues.
                        // - First a pointless database query for existance of the packge which is already guaranteed.
                        // - Second, all the recursive removal related issues. We reinstall a package and there's no need to check for dependency
                        // break as we will be installing it back again.
                        actions.insert(Action::Remove(local_package), ());
                    }
                    ReinstallOptions::Update => {
                        let remote_is_newer = match remote_is_newer(&remote_package, &local_package)
                        {
                            Ok(res) => res,
                            Err(error) => {
                                return Err(InstallError::VersionParse(error.to_string()))
                            }
                        };

                        if remote_is_newer {
                            actions.insert(Action::Remove(local_package), ());
                        } else {
                            info!(
                                "Package {package_name} is already at latest version. Ignoring..."
                            );
                            return Ok(actions);
                        }
                    }
                    ReinstallOptions::Ignore => {
                        info!("Package {package_name} already installed. Ignoring...");
                        return Ok(actions);
                    }
                }
            }
        }
        Err(error) => return Err(InstallError::Database(error)),
    }

    progress::increment_target(
        ProgressType::Packages,
        remote_package.dependencies.len() as i32,
    )
    .await;

    for dependency in remote_package.dependencies.iter() {
        actions.extend(install_package(dependency, package_finder, reinstall_options, db).await?);

        progress::increment_completed(ProgressType::Packages, 1).await;
    }

    actions.insert(Action::Install(remote_package), ());

    Ok(actions)
}

#[async_recursion(?Send)]
async fn remove_package<EDatabase: Error>(
    package_name: &str,
    recursive: bool,
    db: &mut impl PackagesDb<GetError = EDatabase>,
) -> Result<LinkedHashSet<Action>, RemoveError<EDatabase>> {
    debug!("Generating remove actions for package: {package_name}");

    let mut actions: LinkedHashSet<Action> = LinkedHashSet::new();

    let db_package = match db.get_package(package_name) {
        Ok(package) => {
            if package.is_none() {
                return Err(RemoveError::PackageNotInstalled(String::from(package_name)));
            }
            package.unwrap()
        }
        Err(error) => return Err(RemoveError::DatabaseGet(error)),
    };

    // Only query for depending packages of 1 level depth as it is enough to verify dependency
    // break
    let depending_packages = match get_depending(package_name, db, 1) {
        Ok(depending) => depending,
        Err(error) => return Err(RemoveError::DatabaseGet(error)),
    };

    if !depending_packages.is_empty() {
        if recursive {
            info!("Found depending packages, uninstalling...");
            progress::increment_target(ProgressType::Packages, depending_packages.len() as i32)
                .await;

            for dependency in depending_packages.iter() {
                actions.extend(remove_package(&dependency.package_data.name, recursive, db).await?);

                progress::increment_completed(ProgressType::Packages, 1).await;
            }
        } else {
            let depending_packages: Vec<String> = depending_packages
                .into_iter()
                .map(|p| p.package_data.name)
                .collect();

            return Err(RemoveError::DependencyBreak(
                String::from(package_name),
                depending_packages,
            ));
        }
    }

    let action = Action::Remove(db_package);
    actions.insert(action, ());

    Ok(actions)
}

fn remote_is_newer(
    remote_package: &RemotePackage,
    local_package: &LocalPackage,
) -> Result<bool, semver::Error> {
    let remote_version = Version::parse(&remote_package.package_data.version)?;
    let local_version = Version::parse(&local_package.package_data.version)?;

    Ok(remote_version > local_version)
}

fn get_depending<EDatabase: Error>(
    package_name: &str,
    db: &mut impl PackagesDb<GetError = EDatabase>,
    max_level: i32,
) -> Result<Vec<LocalPackage>, EDatabase> {
    Ok(get_depending_recursive(package_name, db, 0, max_level)?
        .keys()
        .cloned()
        .collect())
}
fn get_depending_recursive<EDatabase: Error>(
    package_name: &str,
    db: &mut impl PackagesDb<GetError = EDatabase>,
    level: i32,
    max_level: i32,
) -> Result<LinkedHashSet<LocalPackage>, EDatabase> {
    if level == max_level {
        return Ok(LinkedHashSet::new());
    }

    let mut result = LinkedHashSet::new();

    let depending = db.get_depending_packages(package_name)?;

    for package in depending.into_iter() {
        result.extend(get_depending_recursive(
            &package.package_data.name,
            db,
            level + 1,
            max_level,
        )?);

        result.insert(package, ());
    }

    Ok(result)
}
