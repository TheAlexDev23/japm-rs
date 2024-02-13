use std::process::exit;

use log::{error, trace};
use logger::StdLogger;

use clap::{ArgAction, Parser, Subcommand};

use action::Action;
use config::Config;
use db::SqlitePackagesDb;

use crate::commands::PackageFinder;

mod action;
mod commands;
mod config;
mod db;
mod logger;
mod package;
mod package_finders;

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

    let config = get_config();

    let mut db = get_db();

    if let Some(command) = args.command {
        let result: Result<Vec<Action>, String> = match command {
            CommandType::Install {
                from_file,
                reinstall,
                packages,
            } => {
                let finder: Box<dyn PackageFinder> = if from_file {
                    Box::new(package_finders::FromFilePackageFinder)
                } else {
                    Box::new(package_finders::RemotePackageFinder::new(&config))
                };

                commands::install_packages(packages, &*finder, reinstall, &mut db)
            }
            CommandType::Remove {
                packages,
                recursive,
            } => commands::remove_packages(packages, recursive, &mut db),
            _ => todo!("Command is unsupported"),
        };

        match result {
            Ok(actions) => {
                trace!("Performing actions:\\n{actions:#?}");
                for action in actions {
                    trace!("Commiting action {action}");
                    if let Err(error_message) = action.commit(&mut db) {
                        error!("Could not commit action:\\n{error_message}");
                    } else {
                        trace!("Commited action");
                    }
                }
            }
            Err(error_message) => {
                error!("Error while performing command:\\n{error_message}");
                exit(-1);
            }
        }
    }
}

fn get_config() -> Config {
    const CONFIG_PATH: &str = "/etc/japm/config.json";

    match Config::create_default_config_if_necessary(CONFIG_PATH) {
        Ok(created) => {
            if created {
                if let Err(error) = Config::write_default_config(CONFIG_PATH) {
                    error!("Could not write default config:\\n{error}");
                    exit(-1);
                }
            }
        },
        Err(error) => {
            error!("Could not create default config if necessary:\\n{error}");
            exit(-1);
        }
    }

    let config = match Config::from_file(CONFIG_PATH) {
        Ok(config) => config,
        Err(error) => {
            match error {
                config::Error::IO(error) => error!("Could not parse config due to an IO error: {error}"),
                config::Error::Json(error) => error!("Could not parse config due to a json eror: {error}"),
                config::Error::Syntax(error_message) => error!("Could not parse config due to invalid structure/parameters: {error_message}"),
            }
            exit(-1);
        }
    };
    config
}

fn get_db() -> SqlitePackagesDb {
     match SqlitePackagesDb::create_db_file_if_necessary() {
        Ok(created) => {
            let mut db = match SqlitePackagesDb::new() {
                Ok(db) => db,
                Err(error) => {
                    error!("Could not connect to database:\n{error}");
                    exit(-1);
                }
            };

            if created {
                if let Err(error) = db.initialize_database() {
                    error!("Could not initialize database:\n{error}");
                    exit(-1);
                }
            } 

            db
        }
        Err(error) => {
            error!("Could not create db file if necessary:\n{error}");
            exit(-1);
        }
    }
}