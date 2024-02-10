use log::{debug, trace, warn};
use shell_words;
use std::{fmt::Display, process::Command};

use super::package::Package;

#[derive(Clone, Debug)]
pub struct Action {
    pub action_type: ActionType,
    pub package: Package,
}

#[derive(Clone, Debug)]
pub enum ActionType {
    Install,
    Remove,
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.action_type {
            ActionType::Install => write!(f, "Install {}", self.package.package_data.name),
            ActionType::Remove => write!(f, "Remove {}", self.package.package_data.name),
        }
    }
}

impl Action {
    pub fn commit(self) -> Result<(), String> {
        debug!("Action commit {self}");
        let command_iter = match self.action_type {
            ActionType::Install => self.package.install.iter(),
            ActionType::Remove => self.package.remove.iter(),
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
