use std::fmt::Display;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum InstallError<EDatabase: Display, EFind: Display> {
    #[error("Package {0} not found.")]
    PackageNotFound(String),
    #[error("Error while searching for package {0}")]
    Find(EFind),
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
