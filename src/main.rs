use log::{error, info, trace};

use clap::{ArgAction, Parser, Subcommand};

use config::Config;
use db::SqlitePackagesDb;
use default_package_finder::DefaultPackageFinder;
use frontends::{StdFrontend, TuiFrontend};
use logger::FrontendLogger;
use progress::{FrontendProgress, Progress, ProgressType};

mod action;
mod commands;
mod config;
mod db;
mod default_package_finder;
mod frontends;
mod logger;
mod package;
mod progress;

#[cfg(test)]
mod test_helpers;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, action=ArgAction::SetTrue)]
    verbose: bool,
    #[arg(long, action=ArgAction::SetTrue)]
    no_tui: bool,
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

    if args.no_tui {
        frontends::set_boxed_frontend(Box::new(
            StdFrontend::new().expect("Could not initialize std frontend."),
        ))
    } else {
        frontends::set_boxed_frontend(Box::new(
            TuiFrontend::init().expect("Could not initialize TUI frontend"),
        ));
        unsafe {
            GATHER_KEY_BEFORE_EXIT = true;
        }
    }

    let mut progress = FrontendProgress::new();

    match log::set_boxed_logger(Box::new(FrontendLogger)) {
        Ok(()) => log::set_max_level(if args.verbose {
            log::LevelFilter::Trace
        } else {
            log::LevelFilter::Info
        }),
        Err(error) => {
            eprintln!("Could not setup logger: {error}");
        }
    };

    progress.increment_target(ProgressType::Setup, 2);

    let config = get_config();
    progress.increment_completed(ProgressType::Setup, 1);

    let mut db = get_db();
    progress.increment_completed(ProgressType::Setup, 1);

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
                    &mut progress,
                    &mut db,
                )
                .map_err(|e| e.to_string())
            }
            CommandType::Remove {
                packages,
                recursive,
            } => commands::remove_packages(packages, recursive, &mut progress, &mut db)
                .map_err(|e| e.to_string()),
            CommandType::Update { system, packages } => {
                let mut package_finder = DefaultPackageFinder::new(false, &config);
                if system {
                    commands::update_all_packages(&mut package_finder, &mut progress, &mut db)
                } else {
                    commands::update_packages(packages, &mut package_finder, &mut progress, &mut db)
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

                // If no actions need to be performed the actions progress would be 0/0 which is
                // treated as 0% completed by the current implementaion. Setting it 1/1 will
                // prevent the end screen having non 100% completion even if everything is
                // completed
                if actions.is_empty() {
                    progress.set_comleted(progress::ProgressType::Actions);
                }

                progress.increment_target(ProgressType::Actions, actions.len() as i32);

                for action in actions {
                    trace!("Commiting action {action}");
                    if let Err(error) = action.commit(&mut db) {
                        error!("Could not commit action:\n{error}");
                    } else {
                        trace!("Commited action");
                    }

                    progress.increment_completed(ProgressType::Actions, 1);
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
