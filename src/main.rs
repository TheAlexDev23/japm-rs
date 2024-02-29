use std::fmt::Display;

use rayon::iter::{IntoParallelIterator, ParallelIterator};
use tokio::join;

use clap::{ArgAction, Parser, Subcommand};

use log::{debug, error, info};

use action::Action;
use config::Config;
use db::{PackagesDb, SqlitePackagesDb};
use logger::FrontendLogger;
use package_finder::DefaultPackageFinder;
use progress::{FrontendProgress, ProgressType};

mod action;
mod commands;
mod config;
mod db;
mod frontends;
mod logger;
mod package;
mod package_finder;
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

#[tokio::main]
async fn main() {
    let args = Args::parse();

    {
        let (write_handle, read_handle) = frontends::messaging::generate_message_pair();
        frontends::set_ui_messenger(write_handle);
        if args.no_tui {
            frontends::stdout::init(read_handle).expect("Could not initialize STD frontend.");
        } else {
            frontends::tui::init(read_handle).expect("Could not initialize TUI frontend.");
            unsafe {
                GATHER_KEY_BEFORE_EXIT = true;
            }
        }
    }

    progress::set_boxed_progress(Box::new(FrontendProgress::new()));

    match log::set_boxed_logger(Box::new(
        FrontendLogger::new().expect("Could not initialize frontend logger."),
    )) {
        Ok(()) => log::set_max_level(if args.verbose {
            log::LevelFilter::Trace
        } else {
            log::LevelFilter::Info
        }),
        Err(error) => {
            eprintln!("Could not setup logger: {error}");
        }
    };

    let (config, mut db) = join!(get_config(), get_db());

    if let Some(command) = args.command {
        debug!("Generating actions for command {command:?}");
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
                .await
                .map_err(|e| e.to_string())
            }
            CommandType::Remove {
                packages,
                recursive,
            } => commands::remove_packages(packages, recursive, &mut db)
                .await
                .map_err(|e| e.to_string()),
            CommandType::Update { system, packages } => {
                let mut package_finder = DefaultPackageFinder::new(false, &config);
                if system {
                    commands::update_all_packages(&mut package_finder, &mut db).await
                } else {
                    commands::update_packages(packages, &mut package_finder, &mut db).await
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
            // TODO: make a pretty actions display screen
            Ok(actions) => {
                if let Err(error) = build_actions(actions.clone()).await {
                    error!("Error while building actions: {error}");
                    exit(-1).await
                }
                if let Err(error) = commit_actions(actions, &mut db).await {
                    error!("Error while commiting actions: {error}");
                    exit(-1).await
                }
            }
            Err(error_message) => {
                error!("Error while performing command:\n{error_message}");
                exit(-1).await
            }
        }
    }

    exit(0).await
}

async fn get_config() -> Config {
    const CONFIG_PATH: &str = "/etc/japm/config.json";

    progress::increment_target(ProgressType::Setup, 1).await;

    match Config::create_default_config_if_necessary(CONFIG_PATH).await {
        Ok(created) => {
            if created {
                if let Err(error) = Config::write_default_config(CONFIG_PATH).await {
                    error!("Could not write default config: {error}");
                    exit(-1).await
                }
            }
        }
        Err(error) => {
            error!("Could not create default config if necessary: {error}");
            exit(-1).await
        }
    }

    match Config::from_file(CONFIG_PATH).await {
        Ok(config) => {
            progress::increment_completed(ProgressType::Setup, 1).await;
            config
        }
        Err(error) => {
            error!("Could not get config: {error}");
            exit(-1).await
        }
    }
}

async fn get_db() -> SqlitePackagesDb {
    progress::increment_target(ProgressType::Setup, 1).await;
    match SqlitePackagesDb::create_db_file_if_necessary().await {
        Ok(created) => {
            let mut db = match SqlitePackagesDb::new() {
                Ok(db) => db,
                Err(error) => {
                    error!("Could not connect to the database: {error}");
                    exit(-1).await
                }
            };

            if created {
                if let Err(error) = db.initialize_database() {
                    error!("Could not initialize database: {error}");
                    exit(-1).await
                }
            }

            progress::increment_completed(ProgressType::Setup, 1).await;
            db
        }
        Err(error) => {
            error!("Could not create db file if necessary: {error}");
            exit(-1).await
        }
    }
}

async fn build_actions(actions: Vec<Action>) -> Result<(), action::BuildError> {
    if actions.is_empty() {
        progress::set_comleted(progress::ProgressType::ActionsBuild).await;
    } else {
        progress::increment_target(ProgressType::ActionsBuild, actions.len() as i32).await;
    }

    let rt = tokio::runtime::Handle::current();
    actions.into_par_iter().try_for_each(|mut action| {
        action.build("/var/lib/japm/install_pkgs/")?;
        rt.spawn(async move {
            frontends::display_action(&action).await;
        });
        Ok(())
    })
}

async fn commit_actions<DB, EDatabaseAdd, EDatabaseRemove>(
    actions: Vec<Action>,
    db: &mut DB,
) -> Result<(), action::CommitError<EDatabaseAdd, EDatabaseRemove>>
where
    EDatabaseAdd: Display,
    EDatabaseRemove: Display,
    DB: PackagesDb<AddError = EDatabaseAdd, RemoveError = EDatabaseRemove>,
{
    if actions.is_empty() {
        progress::set_comleted(progress::ProgressType::ActionsCommit).await;
    } else {
        progress::increment_target(ProgressType::ActionsCommit, actions.len() as i32).await;
    }

    for action in actions {
        action.commit(db)?;
        progress::increment_completed(ProgressType::ActionsCommit, 1).await;
    }

    Ok(())
}

async fn exit(code: i32) -> ! {
    // Due to the async nature of the logging/frontend implementation, we need to make sure all
    // needed messages have logged before showing the "press any key to exit" screen
    std::thread::sleep(std::time::Duration::from_millis(100));

    if unsafe { GATHER_KEY_BEFORE_EXIT } {
        info!("Press any key to exit");
        crossterm::event::read().expect("Could not read input");
    }

    frontends::exit().await;

    std::process::exit(code);
}
