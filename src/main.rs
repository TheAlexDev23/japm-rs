use std::fs;
use std::process::exit;

use clap::{ArgAction, Parser, Subcommand};

use japml::{action::Action, action::ActionType, package::Package};

use logger::Logger;

mod japml;
mod logger;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    /// Command to perform
    command: Option<CommandType>,
}

#[derive(Debug, Subcommand)]
enum CommandType {
    Install {
        #[arg(short, long, action=ArgAction::SetTrue)]
        from_file: bool,
        packages: Vec<String>,
    },
    Remove {
        packages: Vec<String>,
    },
    Update {
        packages: Vec<String>,
    },
    Info {
        packages: Vec<String>,
    },
}

fn main() {
    let args = Args::parse();
    let logger = Logger::new(false, logger::LogLevel::Dev);

    if let Some(command) = args.command {
        let result: Result<Vec<Action>, String> = match command {
            CommandType::Install {
                from_file,
                packages,
            } => install_packages(from_file, packages),
            _ => todo!("Command is unsupported"),
        };

        match result {
            Ok(actions) => {
                for action in actions {
                    if let Err(error) = action.commit(&logger) {
                        logger.crit(format!("Error while commiting actions:\n{error}"))
                    }
                }
            }
            Err(error) => {
                logger.crit(format!("Error while performing command:\n{}", error));
                exit(-1);
            }
        }
    }
}

fn install_packages(from_file: bool, package_names: Vec<String>) -> Result<Vec<Action>, String> {
    let packages_len = package_names.len();

    let mut packages: Vec<Package> = Vec::with_capacity(packages_len);

    for package_name in package_names.iter() {
        match get_package(package_name, from_file) {
            Ok(package) => packages.push(package),
            Err(error) => return Err(error),
        };
    }

    // There is no way to guess how many dependencies a package could have
    let mut actions: Vec<Action> = Vec::new();

    for package in packages.iter() {
        match get_dependencies_recursive(package, &|name| get_package(name, from_file)) {
            Ok(dependencies) => {
                for dependency in dependencies.into_iter() {
                    actions.push(Action {
                        action_type: ActionType::Install,
                        package: dependency,
                    });
                }
            }
            Err(error) => return Err(format!("Error getting package dependencies:\n{error}")),
        }
    }

    Ok(actions)
}

fn get_package(name: &String, from_file: bool) -> Result<Package, String> {
    if from_file {
        let mut path = name.clone();
        if !path.ends_with(".json") {
            path.push_str(".json");
        }

        match fs::read_to_string(path) {
            Ok(json_content) => match Package::from_json(&json_content) {
                Ok(package) => Ok(package),
                Err(error) => Err(format!("Error while parsing package:\n{error}")),
            },
            Err(error) => Err(format!("Error reading file:\n{error}")),
        }
    } else {
        todo!("Non local package parsing is not supported");
    }
}

fn get_dependencies_recursive<F, E>(package: &Package, get_package: &F) -> Result<Vec<Package>, E>
where
    F: Fn(&String) -> Result<Package, E>,
{
    let mut dependencies: Vec<Package> = Vec::new();
    for dependency in package.dependencies.iter() {
        let dependency = get_package(dependency)?;
        dependencies.extend(get_dependencies_recursive(&dependency, get_package)?.into_iter());
    }
    dependencies.push(package.clone());

    Ok(dependencies)
}
