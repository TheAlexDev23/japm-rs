use std::process::exit;

use clap::{ArgAction, Parser, Subcommand};

use config::Config;
use japml::{
    action::Action, action::ActionType, package::searching::PackageSearchOptions, package::Package,
};

use logger::Logger;

mod config;
mod japml;
mod logger;

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
    let log_level: logger::LogLevel = match args.verbose {
        true => logger::LogLevel::Dev,
        false => logger::LogLevel::Inf,
    };

    if let Err(error_message) = Config::create_default_config_if_doesnt_exist() {
        eprintln!(
            "Something went wrong when attempting to verify or create default configs:\n{error_message}"
        );
        exit(-1);
    }

    let logger = Logger::new(false, log_level);
    let config = match Config::from_default_config() {
        Ok(config) => config,
        Err(error) => {
            logger.crit(format!("Error while attempting to load commit:\n{error}"));
            exit(-1);
        }
    };

    if let Some(command) = args.command {
        let result: Result<Vec<Action>, String> = match command {
            CommandType::Install {
                from_file,
                packages,
            } => install_packages(&logger, &config, packages, from_file),
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

fn install_packages(
    logger: &Logger,
    config: &Config,
    package_names: Vec<String>,
    from_file: bool,
) -> Result<Vec<Action>, String> {
    let packages_len = package_names.len();

    let mut packages: Vec<Package> = Vec::with_capacity(packages_len);

    let remotes: Vec<String> = config.remotes.values().cloned().collect();

    logger.dev("Searching initial packages");

    for package_name in package_names.into_iter() {
        logger.dev(format!("Searching initial package {package_name}"));

        let search_options = if from_file {
            PackageSearchOptions::FromFile(&package_name)
        } else {
            PackageSearchOptions::FromRemote {
                name: package_name,
                remotes: remotes.clone(),
            }
        };

        match Package::find_package(search_options, logger) {
            Ok(package) => packages.push(package),
            Err(error) => return Err(format!("Error while installing package: {error}")),
        };
    }

    // There is no way to guess how many dependencies a package could have
    let mut actions: Vec<Action> = Vec::new();

    logger.inf("Searching dependencies");
    for package in packages.iter() {
        match get_dependencies_recursive(package, &|name| {
            if from_file {
                Package::find_package(PackageSearchOptions::FromFile(name), logger)
            } else {
                Package::find_package(
                    PackageSearchOptions::FromRemote {
                        name: name.clone(),
                        remotes: remotes.clone(),
                    },
                    logger,
                )
            }
        }) {
            Ok(dependencies) => {
                logger.dev(format!(
                    "Recursive dependencies for package {}: {:#?}",
                    package.package_data.name, dependencies
                ));
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
