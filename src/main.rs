use clap::{ArgAction, Parser, Subcommand};
use std::process::exit;

use action::Action;
use config::Config;
use db::{PackagesDb, SqlitePackagesDb};
use package::{searching::PackageSearchOptions, RemotePackage};

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

    const CONFIG_PATH: &str = "/etc/japm/config.json";

    if let Err(error) = Config::create_default_config_if_necessary(CONFIG_PATH) {
        error!("Could not create defaul config if necessary:\n{error}");
        exit(-1);
    }

    let config = match Config::from_file(CONFIG_PATH) {
        Ok(config) => config,
        Err(error) => {
            log::error!("Error while attempting to load config:\n{error}");
            exit(-1);
        }
    };

    if let Err(error) = SqlitePackagesDb::create_db_file_if_necessary() {
        error!("Could not create db file if necessary:\n{error}");
        exit(-1);
    }

    let mut db = match SqlitePackagesDb::new() {
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
            CommandType::Remove { packages } => remove_packages(packages, &mut db),
            _ => todo!("Command is unsupported"),
        };

        match result {
            Ok(actions) => {
                for action in actions {
                    trace!("Commiting action {action}");
                    if let Err(error_message) = action.commit(&mut db) {
                        error!("Could not commit action:\n{error_message}");
                    } else {
                        trace!("Commited action");
                    }
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
    db: &impl PackagesDb,
) -> Result<Vec<Action>, String> {
    let packages_len = package_names.len();

    let mut packages: Vec<RemotePackage> = Vec::with_capacity(packages_len);

    info!("Searching initial packages");

    for package_name in package_names.iter() {
        debug!("Querying db to verify that package is not installed for {package_name}");
        if let Ok(_) = db.get_package(package_name) {
            return Err(format!("Package {package_name} is already installed"));
        }
    }

    for package_name in package_names.into_iter() {
        debug!("Searching initial package {package_name}");

        match RemotePackage::find_package(&package_name, &search_options) {
            Ok(package) => packages.push(package),
            Err(error) => return Err(format!("Error while installing package: {error}")),
        };
    }

    // There is no way to guess how many dependencies a package could have
    let mut actions: Vec<Action> = Vec::new();

    info!("Searching dependencies");
    for package in packages.iter() {
        match get_dependencies_recursive(package, &|name| {
            RemotePackage::find_package(name, &search_options)
        }) {
            Ok(dependencies) => {
                trace!(
                    "Recursive dependencies for package {}: {:#?}",
                    package.package_data.name,
                    dependencies
                );
                for dependency in dependencies.into_iter() {
                    let action = Action::Install(dependency);
                    trace!("Adding action:\n{action}");
                    actions.push(action);
                }
            }
            Err(error) => return Err(format!("Error getting package dependencies:\n{error}")),
        }
    }

    Ok(actions)
}

fn remove_packages(
    package_names: Vec<String>,
    db: &mut SqlitePackagesDb,
) -> Result<Vec<Action>, String> {
    info!("Searching initial packages");
    let mut actions: Vec<Action> = Vec::new();

    for package_name in package_names.into_iter() {
        debug!("Searching initial package {package_name}");

        let db_package = match db.get_package(&package_name) {
            Ok(package) => package,
            Err(error) => return Err(format!("Could not get package from db:\n{error}")),
        };

        let action = Action::Remove(db_package);
        trace!("Adding action {action}");
        actions.push(action);
    }

    Ok(actions)
}

fn get_dependencies_recursive<F, E>(
    package: &RemotePackage,
    get_package: &F,
) -> Result<Vec<RemotePackage>, E>
where
    F: Fn(&String) -> Result<RemotePackage, E>,
{
    let mut dependencies: Vec<RemotePackage> = Vec::new();
    for dependency in package.dependencies.iter() {
        let dependency = get_package(dependency)?;
        dependencies.extend(get_dependencies_recursive(&dependency, get_package)?.into_iter());
    }
    dependencies.push(package.clone());

    Ok(dependencies)
}
