use std::fmt::Display;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum InstallError<EDatabase: Display, EFind: Display> {
    #[error("Package {0} not found.")]
    PackageNotFound(String),
    #[error("Error while searching for package {0}")]
    Find(EFind),
    #[error("Could not parse package version: {0}")]
    // semver::Error does not implement PartialEq so cannot be used directly. So instead should be converted to string.
    VersionParse(String),
    #[error("A database error has occured {0}")]
    Database(EDatabase),
}

#[derive(Error, Debug, PartialEq)]
pub enum RemoveError<EDatabase: Display> {
    #[error("Package {0} not installed.")]
    PackageNotInstalled(String),
    #[error("Removing package {0} breaks dependencies {1:?}.")]
    DependencyBreak(String, Vec<String>),
    #[error("Could not get package from databae: {0}")]
    DatabaseGet(EDatabase),
}

#[derive(Error, Debug, PartialEq)]
pub enum UpdateError<EDatabase: Display, EFind: Display> {
    #[error("Could not get package from databae: {0}")]
    DatabaseGet(EDatabase),
    #[error("Could not generate actions to remove packages: {0}")]
    Remove(#[from] RemoveError<EDatabase>),
    #[error("Could not generate actions to install packages: {0}")]
    Install(#[from] InstallError<EDatabase, EFind>),
}
