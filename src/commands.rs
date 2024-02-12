use log::{debug, info, trace};

use crate::action::Action;
use crate::db::PackagesDb;

use crate::package::RemotePackage;

use linked_hash_map::LinkedHashMap;

/// A linked hash set allows the guarantee that each element will be only once and the ability to iterate
/// in the same order items where inserted.
type LinkedHashSet<T> = LinkedHashMap<T, ()>;

#[cfg(test)]
mod tests;

pub trait PackageFinder {
    fn find_package(&self, package_name: &str) -> Result<RemotePackage, String>;
}

pub fn install_packages(
    packages: Vec<String>,
    package_finder: &dyn PackageFinder,
    reinstall: bool,
    db: &mut impl PackagesDb,
) -> Result<Vec<Action>, String> {
    let mut actions: LinkedHashSet<Action> = LinkedHashSet::new();

    for package_name in packages.iter() {
        match install_package(&package_name, package_finder, reinstall, db) {
            Ok(new_actions) => actions.extend(new_actions),
            Err(error) => {
                return Err(format!(
                    "Could not genereate actions for package {package_name}:\n{error}"
                ))
            }
        }
    }

    Ok(actions.keys().map(|k| k.clone()).collect())
}

pub fn remove_packages(
    package_names: Vec<String>,
    recursive: bool,
    db: &mut impl PackagesDb,
) -> Result<Vec<Action>, String> {
    let mut actions: LinkedHashSet<Action> = LinkedHashSet::new();

    for package_name in package_names.into_iter() {
        match remove_package(&package_name, recursive, db) {
            Ok(new_actions) => actions.extend(new_actions),
            Err(error) => {
                return Err(format!(
                    "Could not generate actions for package {package_name}:\n{error}"
                ))
            }
        }
    }

    Ok(actions.keys().map(|k| k.clone()).collect())
}

fn install_package(
    package_name: &str,
    package_finder: &dyn PackageFinder,
    reinstall: bool,
    db: &mut impl PackagesDb,
) -> Result<LinkedHashSet<Action>, String> {
    let mut actions: LinkedHashSet<Action> = LinkedHashSet::new();

    trace!("Generating install actions for package: {package_name}");

    if let Ok(local_package) = db.get_package(package_name) {
        if reinstall {
            info!("Package {package_name} already installed, reinstalling...");
            // It's also possible to call remove_package and get the packge removal specific actions.
            // But this can cause issues.
            // - First a pointless database query for existance of the packge which is already guaranteed.
            // - Second, all the recursive removal related issues. We reinstall a package and there's no need to check for dependency
            // break as we will be installing it back again.
            actions.insert(Action::Remove(local_package), ());
        } else {
            info!("Package {package_name} already installed. Ignoring...");
            return Ok(actions);
        }
    }

    let package = package_finder.find_package(package_name)?;

    trace!("Found remote package:\n{package:#?}");
    for dependency in package.dependencies.iter() {
        actions.extend(install_package(dependency, package_finder, reinstall, db)?);
    }

    actions.insert(Action::Install(package), ());

    Ok(actions)
}

fn remove_package(
    package_name: &str,
    recursive: bool,
    db: &mut impl PackagesDb,
) -> Result<LinkedHashSet<Action>, String> {
    let mut actions: LinkedHashSet<Action> = LinkedHashSet::new();

    debug!("Searching initial package {package_name}");

    let db_package = match db.get_package(&package_name) {
        Ok(package) => package,
        Err(error) => return Err(format!("Could not get package from db:\n{error}")),
    };

    if let Ok(depending_packages) = db.get_depending_packages(package_name) {
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

                return Err(format!(
                    "Removing package breaks dependencies: {depending_packages:?}. "
                ));
            }
        }
    }

    let action = Action::Remove(db_package);
    trace!("Adding action {action}");
    actions.insert(action, ());

    Ok(actions)
}
