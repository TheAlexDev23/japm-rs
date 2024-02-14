use super::*;

use crate::commands;

use crate::test_helpers::MockPackagesDb;
use mock_package_finder::MockPackageFinder;

mod mock_package_finder;

#[test]
fn test_install_simple_package_actions_generated_succesfully() {
    let (mut mock_db, package_finder) = get_mocks();
    let remote_package = package_finder.get_simple_packge();

    let install_result = commands::install_packages(
        vec![remote_package.package_data.name.clone()],
        &package_finder,
        false,
        false,
        &mut mock_db,
    );

    assert!(install_result.is_ok());
    assert!(install_result.unwrap() == vec![Action::Install(remote_package)]);
}

#[test]
fn test_installed_package_is_ignored() {
    let (mut mock_db, package_finder) = get_mocks();
    let remote_package = package_finder.get_simple_packge();

    mock_db.add_package(&remote_package).unwrap();

    let install_result = commands::install_packages(
        vec![remote_package.package_data.name.clone()],
        &package_finder,
        false,
        false,
        &mut mock_db,
    );

    assert!(install_result.is_ok());
    assert!(install_result.unwrap() == vec![]);
}

#[test]
fn test_installed_package_is_reinstalled() {
    let (mut mock_db, package_finder) = get_mocks();
    let remote_package = package_finder.get_simple_packge();

    mock_db.add_package(&remote_package).unwrap();

    let local_packge = mock_db
        .get_package(&remote_package.package_data.name)
        .unwrap()
        .unwrap();

    let install_result = commands::install_packages(
        vec![remote_package.package_data.name.clone()],
        &package_finder,
        false,
        true,
        &mut mock_db,
    );

    assert!(install_result.is_ok());
    assert!(
        install_result.unwrap()
            == vec![
                Action::Remove(local_packge),
                Action::Install(remote_package)
            ]
    );
}

#[test]
fn test_remove_package_non_recursive_with_depending_packges_is_not_allowed() {
    let (mut mock_db, package_finder) = get_mocks();
    let package_with_dependency = package_finder.get_package_with_dependency();
    let package_dependency = package_finder
        .find_package(&package_with_dependency.dependencies[0])
        .unwrap()
        .unwrap();

    mock_db.add_package(&package_dependency).unwrap();
    mock_db.add_package(&package_with_dependency).unwrap();

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
fn test_remove_package_recursive_removes_depending() {
    let (mut mock_db, package_finder) = get_mocks();
    let package_with_dependency = package_finder.get_package_with_dependency();
    let package_dependency = package_finder
        .find_package(&package_with_dependency.dependencies[0])
        .unwrap()
        .unwrap();

    mock_db.add_package(&package_dependency).unwrap();
    mock_db.add_package(&package_with_dependency).unwrap();

    let local_package_with_dependency = mock_db
        .get_package(&package_with_dependency.package_data.name)
        .unwrap()
        .unwrap();
    let local_package_dependency = mock_db
        .get_package(&package_dependency.package_data.name)
        .unwrap()
        .unwrap();

    let remove_result = commands::remove_packages(
        vec![package_dependency.package_data.name],
        true,
        &mut mock_db,
    );

    assert!(remove_result.is_ok());

    assert!(
        remove_result.unwrap()
            == vec![
                Action::Remove(local_package_with_dependency),
                Action::Remove(local_package_dependency)
            ]
    );
}

#[test]
fn test_remove_simple_package_actions_generated_succesfully() {
    let (mut mock_db, package_finder) = get_mocks();
    let remote_package = package_finder.get_simple_packge();

    mock_db.add_package(&remote_package).unwrap();

    let local_package = mock_db
        .get_package(&remote_package.package_data.name.clone())
        .unwrap()
        .unwrap();

    let remove_result =
        commands::remove_packages(vec![remote_package.package_data.name], false, &mut mock_db);

    assert!(remove_result.is_ok());
    assert!(remove_result.unwrap() == vec![Action::Remove(local_package)]);
}

fn get_mocks() -> (MockPackagesDb, MockPackageFinder) {
    (MockPackagesDb::new(), MockPackageFinder)
}
