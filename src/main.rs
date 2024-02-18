use frontends::TuiFrontend;
use log::{error, info, trace};

use clap::{ArgAction, Parser, Subcommand};

use config::Config;
use db::SqlitePackagesDb;
use default_package_finder::DefaultPackageFinder;
use logger::FrontendLogger;

mod action;
mod commands;
mod config;
mod db;
mod default_package_finder;
mod frontends;
mod logger;
mod package;

#[cfg(test)]
mod test_helpers;

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
        #[arg(short, long, action=ArgAction::SetTrue)]
        system: bool,
        packages: Vec<String>,
    },
    Info {
        packages: Vec<String>,
    },
}

static mut GATHER_KEY_BEFORE_EXIT: bool = false;

fn main() {
    let args = Args::parse();

    frontends::set_boxed_frontend(Box::new(
        TuiFrontend::init().expect("Could not initialize TUI frontend"),
    ));

    frontends::refresh();

    unsafe {
        GATHER_KEY_BEFORE_EXIT = true;
    }

    match log::set_boxed_logger(Box::new(FrontendLogger)) {
        Ok(()) => log::set_max_level(log::LevelFilter::Trace),
        Err(error) => {
            eprintln!("Could not setup logger: {error}");
        }
    };

    let config = get_config();

    let mut db = get_db();

    if let Some(command) = args.command {
        let result: Result<Vec<action::Action>, String> = match command {
            CommandType::Install {
                from_file,
                reinstall,
                packages,
            } => {
                let reinstall_options = if reinstall {
                    commands::ReinstallOptions::ForceReinstall
                } else {
                    commands::ReinstallOptions::Ignore
                };

                let mut package_finder = DefaultPackageFinder::new(from_file, &config);

                commands::install_packages(
                    packages,
                    &mut package_finder,
                    &reinstall_options,
                    &mut db,
                )
                .map_err(|e| e.to_string())
            }
            CommandType::Remove {
                packages,
                recursive,
            } => commands::remove_packages(packages, recursive, &mut db).map_err(|e| e.to_string()),
            CommandType::Update { system, packages } => {
                let mut package_finder = DefaultPackageFinder::new(false, &config);
                if system {
                    commands::update_all_packages(&mut package_finder, &mut db)
                } else {
                    commands::update_packages(packages, &mut package_finder, &mut db)
                }
            }
            .map_err(|e| e.to_string()),
            CommandType::Info { packages } => {
                if let Err(error) = commands::print_package_info(packages, &mut db) {
                    Err(error.to_string())
                } else {
                    Ok(vec![])
                }
            }
        };

        match result {
            Ok(actions) => {
                trace!("Performing actions: {actions:#?}");
                for action in actions {
                    trace!("Commiting action {action}");
                    if let Err(error) = action.commit(&mut db) {
                        error!("Could not commit action:\n{error}");
                    } else {
                        trace!("Commited action");
                    }

                    frontends::display_action(&action);
                }
            }
            Err(error_message) => {
                error!("Error while performing command:\n{error_message}");
                exit(-1);
            }
        }
    }

    exit(0);
}

fn get_config() -> Config {
    const CONFIG_PATH: &str = "/etc/japm/config.json";

    match Config::create_default_config_if_necessary(CONFIG_PATH) {
        Ok(created) => {
            if created {
                if let Err(error) = Config::write_default_config(CONFIG_PATH) {
                    error!("Could not write default config: {error}");
                    exit(-1);
                }
            }
        }
        Err(error) => {
            error!("Could not create default config if necessary: {error}");
            exit(-1);
        }
    }

    match Config::from_file(CONFIG_PATH) {
        Ok(config) => config,
        Err(error) => {
            error!("Could not get config: {error}");
            exit(-1);
        }
    }
}

fn get_db() -> SqlitePackagesDb {
    match SqlitePackagesDb::create_db_file_if_necessary() {
        Ok(created) => {
            let mut db = match SqlitePackagesDb::new() {
                Ok(db) => db,
                Err(error) => {
                    error!("Could not connect to the database: {error}");
                    exit(-1);
                }
            };

            if created {
                if let Err(error) = db.initialize_database() {
                    error!("Could not initialize database: {error}");
                    exit(-1);
                }
            }

            db
        }
        Err(error) => {
            error!("Could not create db file if necessary: {error}");
            exit(-1);
        }
    }
}

fn exit(code: i32) -> ! {
    if unsafe { GATHER_KEY_BEFORE_EXIT } {
        info!("Press any key to exit");
        crossterm::event::read().expect("Could not read input");
    }

    frontends::exit();

    std::process::exit(code);
}
