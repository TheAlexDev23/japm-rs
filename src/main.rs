use clap::{ArgAction, Parser, Subcommand};
use std::process::exit;

use action::{Action, ActionType};
use config::Config;
use db::InstalledPackagesDb;
use package::{searching::PackageSearchOptions, Package};

use log::{debug, error, info, trace};
use logger::StdLogger;

mod action;
mod config;
mod db;
mod logger;
mod package;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, action=ArgAction::SetTrue)]
    verbose: bool,
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

    let logger: Box<StdLogger> = Box::default();

    match log::set_boxed_logger(logger) {
        Ok(()) => log::set_max_level(log::LevelFilter::Trace),
        Err(error) => {
            eprintln!("Could not setup logger: {error}");
        }
    };

    let config = match Config::new() {
        Ok(config) => config,
        Err(error) => {
            log::error!("Error while attempting to load config:\n{error}");
            exit(-1);
        }
    };

    let mut db = match InstalledPackagesDb::new() {
        Ok(db) => db,
        Err(error) => {
            log::error!("Error while attempting to get installed packages database:\n{error}");
            exit(-1);
        }
    };

    if let Some(command) = args.command {
        let result: Result<Vec<Action>, String> = match command {
            CommandType::Install {
                from_file,
                packages,
            } => {
                let search_options = if from_file {
                    PackageSearchOptions::FromFile
                } else {
                    PackageSearchOptions::FromRemote(config.remotes.values().cloned().collect())
                };

                install_packages(packages, search_options)
            }
            _ => todo!("Command is unsupported"),
        };

        match result {
            Ok(actions) => {
                for action in actions {
                    trace!("Commiting action {action}");
                    if let Err(error_message) = action.commit(&mut db) {
                        error!("Error while commiting actions:\n{error_message}");
                    }
                    trace!("Commited action");
                }
            }
            Err(error_message) => {
                error!("Error while performing command:\n{error_message}");
                exit(-1);
            }
        }
    }
}

fn install_packages(
    package_names: Vec<String>,
    search_options: PackageSearchOptions,
) -> Result<Vec<Action>, String> {
    let packages_len = package_names.len();

    let mut packages: Vec<Package> = Vec::with_capacity(packages_len);

    info!("Searching initial packages");

    for package_name in package_names.into_iter() {
        debug!("Searching initial package {package_name}");

        match Package::find_package(&package_name, &search_options) {
            Ok(package) => packages.push(package),
            Err(error) => return Err(format!("Error while installing package: {error}")),
        };
    }

    // There is no way to guess how many dependencies a package could have
    let mut actions: Vec<Action> = Vec::new();

    info!("Searching dependencies");
    for package in packages.iter() {
        match get_dependencies_recursive(package, &|name| {
            Package::find_package(name, &search_options)
        }) {
            Ok(dependencies) => {
                trace!(
                    "Recursive dependencies for package {}: {:#?}",
                    package.package_data.name,
                    dependencies
                );
                for dependency in dependencies.into_iter() {
                    let action = Action {
                        action_type: ActionType::Install,
                        package: dependency,
                    };
                    trace!("Adding action:\n{action}");
                    actions.push(action);
                }
            }
            Err(error) => return Err(format!("Error getting package dependencies:\n{error}")),
        }
    }

    Ok(actions)
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
