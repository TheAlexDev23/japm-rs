use std::fmt::Display;

use log::{debug, info, trace};

use crate::action::Action;
use crate::db::PackagesDb;
use crate::package::{LocalPackage, RemotePackage};

pub use errors::*;

use linked_hash_map::LinkedHashMap;

/// A linked hash set allows the guarantee that each element will be only once and the ability to iterate
/// in the same order items where inserted.
type LinkedHashSet<T> = LinkedHashMap<T, ()>;

pub mod errors;
#[cfg(test)]
mod tests;

pub trait PackageFinder {
    type Error: Display;
    fn find_package(&self, package_name: &str) -> Result<Option<RemotePackage>, Self::Error>;
}

// TODO: migrate update and force_reinstall to enum
pub fn install_packages<EFind: Display, EDatabase: Display>(
    packages: Vec<String>,
    package_finder: &impl PackageFinder<Error = EFind>,
    update: bool,
    force_reinstall: bool,
    db: &mut impl PackagesDb<GetError = EDatabase>,
) -> Result<Vec<Action>, InstallError<EDatabase, EFind>> {
    let mut actions: LinkedHashSet<Action> = LinkedHashSet::new();

    for package_name in packages.iter() {
        actions.extend(install_package(
            package_name,
            package_finder,
            update,
            force_reinstall,
            db,
        )?);
    }

    Ok(actions.keys().cloned().collect())
}

pub fn remove_packages<EDatabase: Display>(
    package_names: Vec<String>,
    recursive: bool,
    db: &mut impl PackagesDb<GetError = EDatabase>,
) -> Result<Vec<Action>, RemoveError<EDatabase>> {
    let mut actions: LinkedHashSet<Action> = LinkedHashSet::new();

    for package_name in package_names.into_iter() {
        actions.extend(remove_package(&package_name, recursive, db)?);
    }

    Ok(actions.keys().cloned().collect())
}

pub fn update_packages<EDatabase: Display, EFind: Display>(
    package_names: Vec<String>,
    package_finder: &impl PackageFinder<Error = EFind>,
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

        packages_to_update.push(String::from(package_name));

        actions.extend(install_packages(
            packages_to_update,
            package_finder,
            true,
            false,
            db,
        )?);
    }

    Ok(actions)
}

fn install_package<EFind: Display, EDatabase: Display>(
    package_name: &str,
    package_finder: &impl PackageFinder<Error = EFind>,
    update: bool,
    force_reinstall: bool,
    db: &mut impl PackagesDb<GetError = EDatabase>,
) -> Result<LinkedHashSet<Action>, InstallError<EDatabase, EFind>> {
    let mut actions: LinkedHashSet<Action> = LinkedHashSet::new();
    trace!("Generating install actions for package: {package_name}");

    match db.get_package(package_name) {
        Ok(package) => {
            if let Some(package) = package {
                // TODO: Only reinstall if remote version is newer if update and not force_reinstall is given
                if update || force_reinstall {
                    info!("Package {package_name} already installed, reinstalling...");
                    // It's also possible to call remove_package and get the packge removal specific actions.
                    // But this can cause issues.
                    // - First a pointless database query for existance of the packge which is already guaranteed.
                    // - Second, all the recursive removal related issues. We reinstall a package and there's no need to check for dependency
                    // break as we will be installing it back again.
                    actions.insert(Action::Remove(package), ());
                } else {
                    info!("Package {package_name} already installed. Ignoring...");
                    return Ok(actions);
                }
            }
        }
        Err(error) => return Err(InstallError::Database(error)),
    }

    let package = match package_finder.find_package(package_name) {
        Ok(package) => package,
        Err(error) => return Err(InstallError::Find(error)),
    };

    if package.is_none() {
        return Err(InstallError::PackageNotFound(String::from(package_name)));
    }

    let package = package.unwrap();

    trace!("Found remote package:\n{package:#?}");
    for dependency in package.dependencies.iter() {
        actions.extend(install_package(
            dependency,
            package_finder,
            update,
            force_reinstall,
            db,
        )?);
    }

    actions.insert(Action::Install(package), ());

    Ok(actions)
}

fn remove_package<EDatabase: Display>(
    package_name: &str,
    recursive: bool,
    db: &mut impl PackagesDb<GetError = EDatabase>,
) -> Result<LinkedHashSet<Action>, RemoveError<EDatabase>> {
    let mut actions: LinkedHashSet<Action> = LinkedHashSet::new();

    debug!("Searching initial package {package_name}");

    let db_package = match db.get_package(package_name) {
        Ok(package) => {
            if package.is_none() {
                return Err(RemoveError::PackageNotInstalled(String::from(package_name)));
            }
            package.unwrap()
        }
        Err(error) => return Err(RemoveError::DatabaseGet(error)),
    };

    let depending_packages = match get_depending(package_name, db, 1) {
        Ok(depending) => depending,
        Err(error) => return Err(RemoveError::DatabaseGet(error)),
    };

    if !depending_packages.is_empty() {
        if recursive {
            info!("Found depending packages, uninstalling...");
            for dependency in depending_packages.iter() {
                actions.extend(remove_package(
                    &dependency.package_data.name,
                    recursive,
                    db,
                )?);
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
    trace!("Adding action {action}");
    actions.insert(action, ());

    Ok(actions)
}

fn get_depending<EDatabase: Display>(
    package_name: &str,
    db: &mut impl PackagesDb<GetError = EDatabase>,
    max_level: i32,
) -> Result<Vec<LocalPackage>, EDatabase> {
    Ok(get_depending_recursive(package_name, db, 0, max_level)?
        .keys()
        .cloned()
        .collect())
}
fn get_depending_recursive<EDatabase: Display>(
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
