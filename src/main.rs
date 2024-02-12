use std::process::exit;

use log::{error, trace};
use logger::StdLogger;

use clap::{ArgAction, Parser, Subcommand};

use action::Action;
use config::Config;
use db::SqlitePackagesDb;
use package::{searching::PackageSearchOptions, RemotePackage};

mod action;
mod commands;
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
        #[arg(long, action=ArgAction::SetTrue)]
        from_file: bool,
        #[arg(short, long, action=ArgAction::SetTrue)]
        reinstall: bool,
        packages: Vec<String>,
    },
    Remove {
        #[arg(short, long, action=ArgAction::SetTrue)]
        recursive: bool,
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
                reinstall,
                packages,
            } => {
                let search_options = if from_file {
                    PackageSearchOptions::FromFile
                } else {
                    PackageSearchOptions::FromRemote(config.remotes.values().cloned().collect())
                };

                commands::install_packages(packages, search_options, reinstall, &mut db)
            }
            CommandType::Remove {
                packages,
                recursive,
            } => commands::remove_packages(packages, recursive, &mut db),
            _ => todo!("Command is unsupported"),
        };

        match result {
            Ok(actions) => {
                trace!("Performing actions:\n{actions:#?}");
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
