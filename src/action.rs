use log::{debug, trace, warn};
use std::{fmt::Display, process::Command};

use crate::db::PackagesDb;
use crate::package::{LocalPackage, RemotePackage};

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

impl Action {
    pub fn commit(self, db: &mut impl PackagesDb) -> Result<(), String> {
        debug!("Action commit {self}");
        let command_iter = match self {
            Action::Install(ref package) => package.install.iter(),
            Action::Remove(ref package) => package.remove.iter(),
        };

        for command in command_iter {
            debug!("Running command {command}");
            match run_command(command) {
                Ok((stdout, stderr)) => {
                    if !stdout.is_empty() {
                        debug!("out: {stdout}");
                    }
                    if !stderr.is_empty() {
                        warn!("err: {stderr}");
                    }
                }
                Err(error) => {
                    return Err(format!("Error while commiting action {}:\n{error}", self))
                }
            }
        }

        match self {
            Action::Install(package) => {
                if let Err(error) = db.add_package(&package) {
                    return Err(format!("Could not add package to local database:\n{error}"));
                }
            }
            Action::Remove(package) => {
                if let Err(error) = db.remove_package(&package.package_data.name) {
                    return Err(format!("Could not remove package from database:\n{error}"));
                }
            }
        };

        Ok(())
    }
}

fn run_command(command: &str) -> Result<(String, String), String> {
    match shell_words::split(command) {
        Ok(args) => {
            if args.is_empty() {
                return Err(String::from(
                    "Error while attempting to run command. Cannot contain 0 arguments.",
                ));
            }

            trace!("Command as arguments: {args:?}");

            let args_iter = args.iter();

            // .iter().next() instead of get(0) is necessary to consume first item of iter
            #[allow(clippy::all)]
            let mut command = Command::new(args.iter().next().unwrap());

            for arg in args_iter {
                command.arg(arg);
            }

            match command.output() {
                Ok(result) => {
                    if !result.status.success() {
                        match result.status.code() {
                            Some(code) => {
                                return Err(format!("Command failed with exit code {}", code))
                            }
                            None => return Err(String::from("Command failed without exit code")),
                        }
                    }

                    Ok((
                        String::from_utf8_lossy(&result.stdout).to_string(),
                        String::from_utf8_lossy(&result.stderr).to_string(),
                    ))
                }
                Err(error) => Err(format!("Error while running command:\n{error}")),
            }
        }
        Err(error) => Err(format!("Error while parsing command arguments:\n{error}")),
    }
}
