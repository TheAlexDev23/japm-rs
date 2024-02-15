use super::*;

use crate::commands;

use crate::test_helpers::MockPackagesDb;
use mock_package_finder::MockPackageFinder;

mod mock_package_finder;

#[test]
fn test_install_actions_generated_succesfully() {
    let (mut mock_db, mut package_finder) = get_mocks();
    let remote_package = package_finder.get_simple_packge();

    let install_result = commands::install_packages(
        vec![remote_package.package_data.name.clone()],
        &mut package_finder,
        &ReinstallOptions::Ignore,
        &mut mock_db,
    );

    assert_actions(install_result, vec![Action::Install(remote_package)]);
}

#[test]
fn test_remove_package_actions_generated_succesfully() {
    let (mut mock_db, mut package_finder) = get_mocks();
    let remote_package = package_finder.get_simple_packge();

    let local_package = mock_install(&mut mock_db, &remote_package);

    let remove_result =
        commands::remove_packages(vec![remote_package.package_data.name], false, &mut mock_db);

    assert_actions(remove_result, vec![Action::Remove(local_package)]);
}

#[test]
fn test_installed_package_is_ignored() {
    let (mut mock_db, mut package_finder) = get_mocks();
    let remote_package = package_finder.get_simple_packge();

    mock_install(&mut mock_db, &remote_package);

    let install_result = commands::install_packages(
        vec![remote_package.package_data.name.clone()],
        &mut package_finder,
        &ReinstallOptions::Ignore,
        &mut mock_db,
    );

    assert_actions(install_result, vec![]);
}

#[test]
fn test_installed_package_is_updated() {
    let (mut mock_db, mut package_finder) = get_mocks();
    let remote_package = package_finder.get_simple_packge();

    let package_name = remote_package.package_data.name.clone();

    let local_packge = mock_install(&mut mock_db, &remote_package);

    package_finder.update_remote_package_version(&package_name);
    let remote_package = package_finder.get_simple_packge();

    let install_result = commands::install_packages(
        vec![package_name],
        &mut package_finder,
        &ReinstallOptions::Update,
        &mut mock_db,
    );

    assert_actions(
        install_result,
        vec![
            Action::Remove(local_packge),
            Action::Install(remote_package),
        ],
    );
}

#[test]
fn test_latest_ver_installed_package_is_ignored() {
    let (mut mock_db, mut package_finder) = get_mocks();
    let remote_package = package_finder.get_simple_packge();

    mock_install(&mut mock_db, &remote_package);

    let install_result = commands::install_packages(
        vec![remote_package.package_data.name.clone()],
        &mut package_finder,
        &ReinstallOptions::Update,
        &mut mock_db,
    );

    assert_actions(install_result, vec![]);
}

#[test]
fn test_installed_package_is_reinstalled() {
    let (mut mock_db, mut package_finder) = get_mocks();
    let remote_package = package_finder.get_simple_packge();

    let local_package = mock_install(&mut mock_db, &remote_package);

    let install_result = commands::install_packages(
        vec![remote_package.package_data.name.clone()],
        &mut package_finder,
        &ReinstallOptions::ForceReinstall,
        &mut mock_db,
    );

    assert_actions(
        install_result,
        vec![
            Action::Remove(local_package),
            Action::Install(remote_package),
        ],
    );
}

#[test]
fn test_remove_package_with_depending_packages_is_not_allowed() {
    let (mut mock_db, mut package_finder) = get_mocks();
    let package_with_dependency = package_finder.get_package_with_dependency();
    let package_dependency = package_finder
        .find_package(&package_with_dependency.dependencies[0])
        .unwrap()
        .unwrap();

    mock_install(&mut mock_db, &package_dependency);
    mock_install(&mut mock_db, &package_with_dependency);

    let remove_result = commands::remove_packages(
        vec![package_dependency.package_data.name],
        false,
        &mut mock_db,
    );

    assert!(remove_result.is_err());
    assert!(matches!(
        remove_result.unwrap_err(),
        RemoveError::DependencyBreak(_, _)
    ));
}

#[test]
fn test_remove_package_removes_depending() {
    let (mut mock_db, mut package_finder) = get_mocks();
    let package_with_dependency = package_finder.get_package_with_dependency();
    let package_dependency = package_finder
        .find_package(&package_with_dependency.dependencies[0])
        .unwrap()
        .unwrap();

    mock_db.add_package(&package_dependency).unwrap();
    mock_db.add_package(&package_with_dependency).unwrap();

    let local_package_dependency = mock_install(&mut mock_db, &package_dependency);
    let local_package_with_dependency = mock_install(&mut mock_db, &package_with_dependency);

    let remove_result = commands::remove_packages(
        vec![package_dependency.package_data.name],
        true,
        &mut mock_db,
    );

    assert_actions(
        remove_result,
        vec![
            Action::Remove(local_package_with_dependency),
            Action::Remove(local_package_dependency),
        ],
    );
}

fn assert_actions<Error: std::fmt::Debug>(
    result: Result<Vec<Action>, Error>,
    expected_actions: Vec<Action>,
) {
    assert!(result.is_ok());
    assert!(result.unwrap() == expected_actions);
}

fn mock_install(db: &mut MockPackagesDb, remote_package: &RemotePackage) -> LocalPackage {
    db.add_package(remote_package)
        .expect("Could not add mock package to db");

    db.get_package(&remote_package.package_data.name.clone())
        .unwrap()
        .unwrap()
}

fn get_mocks() -> (MockPackagesDb, MockPackageFinder) {
    (MockPackagesDb::new(), MockPackageFinder::new())
}
