use std::fmt::Display;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use log::{debug, info, trace, warn};

use thiserror::Error;

use crate::db::PackagesDb;
use crate::package::{LocalPackage, RemotePackage};

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Action {
    Install(RemotePackage),
    Remove(LocalPackage),
}
impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Install(package) => write!(f, "Install {}", package.package_data.name),
            Action::Remove(package) => write!(f, "Remove {}", package.package_data.name),
        }
    }
}

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("Could not parse command: {0}")]
    Parse(#[from] shell_words::ParseError),

    #[error("An IO error has occured: {0}")]
    IO(#[from] io::Error),

    #[error("Command {0} is invalid: {0}")]
    InvalidCommand(String, String),

    #[error("Command {0} failed with exit code {1} and stderr:\n{2}")]
    CommandFail(String, i32, String),
}

#[derive(Error, Debug)]
pub enum CommitError<EDatabaseAdd: Display, EDatabaseRemove: Display> {
    #[error("Failed to add package to database:\n{0}")]
    DatabaseAdd(EDatabaseAdd),

    #[error("Failed to remove package from database:\n{0}")]
    DatabaseRemove(EDatabaseRemove),
}

impl Action {
    pub fn build(&mut self, package_build_path: &str) -> Result<(), BuildError> {
        match self {
            Action::Install(ref mut package) => {
                install_package(package, package_build_path)?;
            }
            Action::Remove(ref mut package) => {
                remove_package(package)?;
            }
        };

        Ok(())
    }

    pub fn commit<EDatabaseAdd: Display, EDatabaseRemove: Display>(
        &self,
        db: &mut impl PackagesDb<AddError = EDatabaseAdd, RemoveError = EDatabaseRemove>,
    ) -> Result<(), CommitError<EDatabaseAdd, EDatabaseRemove>> {
        match self {
            Action::Install(ref package) => {
                if let Err(error) = db.add_package(package) {
                    return Err(CommitError::DatabaseAdd(error));
                }
            }
            Action::Remove(ref package) => {
                if let Err(error) = db.remove_package(&package.package_data.name) {
                    return Err(CommitError::DatabaseRemove(error));
                }
            }
        };

        Ok(())
    }
}

fn install_package(
    package: &mut RemotePackage,
    package_build_path: &str,
) -> Result<(), BuildError> {
    let install_directory = format!("{}/{}", package_build_path, package.package_data.name);

    if fs::metadata(&install_directory).is_ok() {
        fs::remove_dir_all(&install_directory)?;
    }
    fs::create_dir_all(&install_directory)?;

    run_commands(&package.pre_install, &install_directory)?;

    run_commands(&package.install, &install_directory)?;

    let path_install_directory = Path::new(&install_directory);
    let package_files = find_package_files(
        path_install_directory,
        path_install_directory,
        Path::new("/"),
    )?;

    trace!("Detected package files: {package_files:#?}");

    install_package_files(&package_files)?;
    package.package_files = package_files
        .into_iter()
        .map(|group| group.1.to_string_lossy().into_owned())
        .collect();

    run_commands(&package.post_install, &install_directory)?;

    Ok(())
}

fn remove_package(package: &LocalPackage) -> Result<(), BuildError> {
    run_commands(&package.pre_remove, "/")?;
    delete_package_files(&package.package_files)?;
    run_commands(&package.post_remove, "/")?;

    Ok(())
}

/// Find the files located in `path` that do not exist in `root_path`, and returns an array of
/// original paths and their non-existing root translated equivalents.
///
/// For example, given normal parameters (root_path is `/`), then if an empty usr subdirectory exists
/// in `path`, it won't be included in the result as `/usr` already exists on most filesystems.
/// However, if `path` has `./usr/bin/some_application_name/` subdirectories, then it will be included (given
/// that `some_application_name` is not installed and `/usr/bin/some_application_name/` does not exist)
fn find_package_files(
    path: &Path,
    base_path: &Path,
    root_path: &Path,
) -> Result<Vec<(PathBuf, PathBuf)>, io::Error> {
    let mut new_dirs = Vec::new();
    for subpath in fs::read_dir(path)? {
        let subpath = subpath?.path();
        let translated_subpath = translate_to_root(&subpath, base_path, root_path);

        if !Path::try_exists(&translated_subpath)? {
            new_dirs.push((subpath, translated_subpath));
            continue;
        }

        if subpath.is_dir() {
            new_dirs.extend(find_package_files(&subpath, base_path, root_path)?);
        }
    }

    Ok(new_dirs)
}

fn translate_to_root(file: &Path, files_root_dir: &Path, root_dir: &Path) -> PathBuf {
    let relative = file
        .strip_prefix(files_root_dir)
        .expect("Could not replace prefix");

    root_dir.join(relative)
}

fn install_package_files(package_files: &[(PathBuf, PathBuf)]) -> Result<(), io::Error> {
    for path_group in package_files {
        let source = &path_group.0;
        let dest = &path_group.1;

        trace!("Moving {:?} to {:?}", source, dest);
        fs::rename(source, dest)?;
    }

    Ok(())
}

fn delete_package_files(package_files: &[String]) -> Result<(), io::Error> {
    for path in package_files {
        info!("Deleting path {:?}", path);
        if Path::is_dir(Path::new(&path)) {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

fn run_commands(commands: &Vec<String>, directory: &str) -> Result<(), BuildError> {
    for command in commands {
        debug!("Running command {command}");

        let (stdout, stderr) = run_command(command, directory)?;

        if !stdout.is_empty() {
            debug!("out: {stdout}");
        }
        if !stderr.is_empty() {
            warn!("err: {stderr}");
        }
    }

    Ok(())
}

fn run_command(command: &str, directory: &str) -> Result<(String, String), BuildError> {
    let args = shell_words::split(command)?;
    if args.is_empty() {
        return Err(BuildError::InvalidCommand(
            String::from(command),
            String::from("Cannot have 0 arguments"),
        ));
    }

    trace!("Command as arguments: {args:?}");

    let mut args_iter = args.iter();

    let mut command_proc = Command::new(args_iter.next().unwrap());

    for arg in args_iter {
        command_proc.arg(arg);
    }

    let result = command_proc.current_dir(directory).output()?;

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();

    if !result.status.success() {
        match result.status.code() {
            Some(code) => return Err(BuildError::CommandFail(String::from(command), code, stderr)),
            None => {
                return Err(BuildError::CommandFail(
                    String::from(command),
                    80085,
                    String::from("Command failed but could not get the status code."),
                ))
            }
        }
    }

    Ok((stdout, stderr))
}
