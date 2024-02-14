/// When testing actions, it's important to not test if they react to certain db states, as for example
/// not installing when a package is already installed or
use super::*;

use crate::package::{PackageData, RemotePackage};

use crate::test_helpers::MockPackagesDb;

#[test]
fn test_package_installs() {
    let remote_package = get_mock_remote_package();
    let mut mock_db = MockPackagesDb::new();

    let action = Action::Install(remote_package.clone());

    assert!(action.commit(&mut mock_db).is_ok());

    // If packge is not found Err is returned
    assert!(mock_db
        .get_package(&remote_package.package_data.name)
        .is_ok());
}

#[test]
fn test_package_removes() {
    let remote_package = get_mock_remote_package();
    let mut mock_db = MockPackagesDb::new();

    let package_name = remote_package.package_data.name.clone();

    mock_db.add_package(&remote_package).unwrap();

    let local_package = mock_db.get_package(&package_name).unwrap().unwrap();

    let action = Action::Remove(local_package);

    assert!(action.commit(&mut mock_db).is_ok());
    assert!(mock_db.get_package(&package_name).unwrap().is_none());
}

fn get_mock_remote_package() -> RemotePackage {
    let remote_package = RemotePackage {
        package_data: PackageData {
            name: String::from("test-package"),
            ..Default::default()
        },
        ..Default::default()
    };

    remote_package
}
