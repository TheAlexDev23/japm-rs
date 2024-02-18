use std::fmt::Display;
use std::io;
use std::process::Command;

use log::{debug, trace, warn};

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
pub enum Error<EDatabaseAdd: Display, EDatabaseRemove: Display> {
    #[error("Could not parse command: {0}")]
    Parse(#[from] shell_words::ParseError),

    #[error("Could not read output: {0}")]
    IO(#[from] io::Error),

    #[error("Command {0} is invalid: {0}")]
    InvalidCommand(String, String),

    #[error("Command {0} failed with exit code {1} and stderr:\n{2}")]
    CommandFail(String, i32, String),

    #[error("Failed to add package to database:\n{0}")]
    DatabaseAdd(EDatabaseAdd),

    #[error("Failed to remove package from database:\n{0}")]
    DatabaseRemove(EDatabaseRemove),
}

impl Action {
    pub fn commit<EDatabaseAdd: Display, EDatabaseRemove: Display>(
        &self,
        db: &mut impl PackagesDb<AddError = EDatabaseAdd, RemoveError = EDatabaseRemove>,
    ) -> Result<(), Error<EDatabaseAdd, EDatabaseRemove>> {
        debug!("Action commit {self}");
        let command_iter = match self {
            Action::Install(ref package) => package.install.iter(),
            Action::Remove(ref package) => package.remove.iter(),
        };

        for command in command_iter {
            debug!("Running command {command}");
            let (stdout, stderr) = run_command(command)?;
            if !stdout.is_empty() {
                debug!("out: {stdout}");
            }
            if !stderr.is_empty() {
                warn!("err: {stderr}");
            }
        }

        match self {
            Action::Install(package) => {
                if let Err(error) = db.add_package(package) {
                    return Err(Error::DatabaseAdd(error));
                }
            }
            Action::Remove(package) => {
                if let Err(error) = db.remove_package(&package.package_data.name) {
                    return Err(Error::DatabaseRemove(error));
                }
            }
        };

        Ok(())
    }
}

fn run_command<EDatabaseAdd: Display, EDatabaseRemove: Display>(
    command: &str,
) -> Result<(String, String), Error<EDatabaseAdd, EDatabaseRemove>> {
    let args = shell_words::split(command)?;
    if args.is_empty() {
        return Err(Error::InvalidCommand(
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

    let result = command_proc.output()?;

    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();

    if !result.status.success() {
        match result.status.code() {
            Some(code) => return Err(Error::CommandFail(String::from(command), code, stderr)),
            None => {
                return Err(Error::CommandFail(
                    String::from(command),
                    80085,
                    String::from("Command failed but could not get the status code."),
                ))
            }
        }
    }

    Ok((stdout, stderr))
}
