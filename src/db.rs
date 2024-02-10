use std::fs::{self, File};
use std::path::Path;

use log::trace;

use crate::package::Package;

use diesel::sqlite::SqliteConnection;
use diesel::{table, Connection, Insertable, Queryable, RunQueryDsl};

pub struct InstalledPackagesDb {
    connection: SqliteConnection,
}

table! {
    packages {
        id -> Integer,
        name -> Text,
        version -> Text,
        description -> Text,
        remove_instructions -> Text,
    }
}

// Define the structure representing a package
#[derive(Queryable, Insertable, Debug)]
#[diesel(table_name = packages)]
struct DatabasePackage {
    name: String,
    version: String,
    description: String,
    /// Json array of remove instructions
    remove_instructions: String,
}

impl TryFrom<&Package> for DatabasePackage {
    type Error = String;

    fn try_from(package: &Package) -> Result<Self, Self::Error> {
        Ok(DatabasePackage {
            name: package.package_data.name.clone(),
            version: package.package_data.version.clone(),
            description: package.package_data.description.clone(),
            remove_instructions: match serde_json::to_string(&package.install) {
                Ok(string) => string,
                Err(error) => {
                    return Err(format!(
                        "Could not convert package's install instructions to json:\n{error}"
                    ))
                }
            },
        })
    }
}

const DATABASE_SOURCE: &str = "/var/lib/japm/packages.db";
impl InstalledPackagesDb {
    pub fn new() -> Result<InstalledPackagesDb, String> {
        if let Err(error) = create_db_file_if_necessary() {
            return Err(format!(
                "Error while attempting to create database if necessary:\n{error}"
            ));
        }

        let mut url = String::from("sqlite://");
        url.push_str(DATABASE_SOURCE);

        trace!("Establishing SQL connection with source:\n{url}");

        let mut connection = match SqliteConnection::establish(&url) {
            Ok(connection) => connection,
            Err(error) => {
                return Err(format!(
                    "Could not establish connection to database:\n{error}"
                ))
            }
        };

        const CREATE_TABLE_QUERY: &str = "CREATE TABLE IF NOT EXISTS packages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                description TEXT,
                remove_instructions TEXT
            )";

        trace!("Executing SQL create table query:\n{CREATE_TABLE_QUERY}");

        if let Err(error) = diesel::sql_query(CREATE_TABLE_QUERY).execute(&mut connection) {
            return Err(format!("Could not execute creat table query:\n{error}"));
        }

        Ok(InstalledPackagesDb { connection })
    }

    pub fn add_package(&mut self, package: &Package) -> Result<(), String> {
        use self::packages::dsl::*;

        let db_package: DatabasePackage = package.try_into()?;

        trace!("Inserting {db_package:#?} into the database");

        if let Err(error) = diesel::insert_into(packages)
            .values(db_package)
            .execute(&mut self.connection)
        {
            return Err(format!("Could not insert package to database:\n{error}"));
        }
        Ok(())
    }
}

pub fn create_db_file_if_necessary() -> Result<(), String> {
    trace!("Creating db file if necessary");

    let database_path = Path::new(DATABASE_SOURCE);
    match database_path.try_exists() {
        Ok(exists) => {
            if exists {
                return Ok(());
            }

            trace!("Creating database parent directory");

            // Hardcoded directory allways has parent, unwrap is ok
            if let Err(error) = fs::create_dir_all(database_path.parent().unwrap()) {
                return Err(format!(
                    "Could not create database's directory/ies:\n{error}"
                ));
            }

            trace!("Creating database file");
            if let Err(error) = File::create(DATABASE_SOURCE) {
                return Err(format!("Could not create database file:\n{error}"));
            }

            Ok(())
        }
        Err(error) => Err(format!("Could not verify if database exists:\n{error}")),
    }
}
