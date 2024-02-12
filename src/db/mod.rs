use std::fs::{self, File};
use std::path::Path;

use log::trace;

use crate::package::{LocalPackage, PackageData, RemotePackage};

use diesel::prelude::*;

pub trait PackagesDb {
    fn add_package(&mut self, package: &RemotePackage) -> Result<(), String>;
    fn remove_package(&mut self, package_name: &str) -> Result<(), String>;
    fn get_package(&mut self, name: &str) -> Result<LocalPackage, String>;
    fn get_all_packages(&mut self) -> Result<Vec<LocalPackage>, String>;
    fn get_depending_packages(&mut self, package_name: &str) -> Result<Vec<LocalPackage>, String>;
}

pub struct SqlitePackagesDb {
    connection: SqliteConnection,
}

table! {
    packages {
        id -> Integer,
        name -> Text,
        version -> Text,
        description -> Text,
        remove_instructions -> Text,
        dependencies -> Text,
    }
}

#[derive(Insertable, Debug)]
#[diesel(table_name = packages)]
/// Represens a new package to add to the package database
struct AddPackage {
    name: String,
    version: String,
    description: String,
    ///  Json array of remove instructions
    remove_instructions: String,
    /// Json array of dependencies' names
    dependencies: String,
}

#[derive(Queryable, Debug)]
#[diesel(table_name = packages)]
/// Represents a queryable package from the package database.
struct GetPackage {
    /// Id is generally not used as packages are accessed with strings in the database
    _id: i32,
    pub name: String,
    pub version: String,
    pub description: String,
    /// Json array of remove instructions
    pub remove_instructions: String,
    /// Json array of dependencies' names
    dependencies: String,
}

impl SqlitePackagesDb {
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

    pub fn new() -> Result<SqlitePackagesDb, String> {
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
                remove_instructions TEXT,
                dependencies TEXT
            )";

        trace!("Executing SQL create table query:\n{CREATE_TABLE_QUERY}");

        if let Err(error) = diesel::sql_query(CREATE_TABLE_QUERY).execute(&mut connection) {
            return Err(format!("Could not execute creat table query:\n{error}"));
        }

        Ok(SqlitePackagesDb { connection })
    }
}

const DATABASE_SOURCE: &str = "/var/lib/japm/packages.db";
impl PackagesDb for SqlitePackagesDb {
    fn add_package(&mut self, package: &RemotePackage) -> Result<(), String> {
        use self::packages::dsl::*;

        let db_package: AddPackage = package.try_into()?;

        trace!("Inserting {db_package:#?} into the database");

        if let Err(error) = diesel::insert_into(packages)
            .values(db_package)
            .execute(&mut self.connection)
        {
            return Err(format!("Could not insert package to database:\n{error}"));
        }
        Ok(())
    }

    fn remove_package(&mut self, package_name: &str) -> Result<(), String> {
        use self::packages::dsl::*;

        if let Err(error) =
            diesel::delete(packages.filter(name.eq(package_name))).execute(&mut self.connection)
        {
            return Err(format!("Error while running remove query:\n{error}"));
        }

        Ok(())
    }

    fn get_package(&mut self, package_name: &str) -> Result<LocalPackage, String> {
        use self::packages::dsl::*;

        match packages
            .filter(name.eq(package_name))
            .first::<GetPackage>(&mut self.connection)
        {
            Ok(package) => match <GetPackage as TryInto<LocalPackage>>::try_into(package) {
                Ok(package) => Ok(package),
                Err(error) => Err(format!(
                    "Could not convert query type into package type:\n{error}"
                )),
            },
            Err(error) => {
                if error == diesel::NotFound {
                    Err(String::from("No such package"))
                } else {
                    Err(format!("Error attempting to retrieve package:\n{error}"))
                }
            }
        }
    }

    fn get_all_packages(&mut self) -> Result<Vec<LocalPackage>, String> {
        use self::packages::dsl::*;

        match packages
            .select(packages::all_columns())
            .load::<GetPackage>(&mut self.connection)
        {
            Ok(all_packages) => {
                let convert_into = |item: GetPackage| -> Result<LocalPackage, String> {
                    let package_name = item.name.clone();
                    match item.try_into() {
                        Ok(package) => Ok(package),
                        // If there's an error we can print the specific package it happened to.
                        Err(error) => Err(format!(
                            "Could not convert query package {} into package:\n{}",
                            package_name, error
                        )),
                    }
                };

                let all_packages: Result<Vec<LocalPackage>, String> =
                    all_packages.into_iter().map(convert_into).collect();

                match all_packages {
                    Ok(all_packages) => Ok(all_packages),
                    Err(error) => Err(format!(
                        "Could not map all query types to package type:\n{error}"
                    )),
                }
            }
            Err(error) => Err(format!("Could not get all packages:\n{error}")),
        }
    }

    fn get_depending_packages(&mut self, package_name: &str) -> Result<Vec<LocalPackage>, String> {
        let all_packages = self.get_all_packages()?;
        let mut depending_packages: Vec<LocalPackage> = Vec::new();

        let package_name = String::from(package_name);

        for package in all_packages.into_iter() {
            if package.dependencies.contains(&package_name) {
                depending_packages.push(package);
            }
        }

        Ok(depending_packages)
    }
}

impl TryFrom<&RemotePackage> for AddPackage {
    type Error = String;

    fn try_from(package: &RemotePackage) -> Result<Self, Self::Error> {
        Ok(AddPackage {
            name: package.package_data.name.clone(),
            version: package.package_data.version.clone(),
            description: package.package_data.description.clone(),
            remove_instructions: match serde_json::to_string(&package.remove) {
                Ok(string) => string,
                Err(error) => {
                    return Err(format!(
                        "Could not convert package's install instructions to json:\n{error}"
                    ))
                }
            },
            dependencies: match serde_json::to_string(&package.dependencies) {
                Ok(string) => string,
                Err(error) => {
                    return Err(format!(
                        "Could not convert the package's dependencies to json:\n{error}"
                    ))
                }
            },
        })
    }
}

impl TryInto<LocalPackage> for GetPackage {
    type Error = String;

    fn try_into(self) -> Result<LocalPackage, Self::Error> {
        Ok(LocalPackage {
            package_data: PackageData {
                name: self.name,
                version: self.version,
                description: self.description,
            },
            remove_instructions: match serde_json::from_str(&self.remove_instructions) {
                Ok(result) => result,
                Err(error) => return Err(format!("Could not parse remove instructions:\n{error}")),
            },
            dependencies: match serde_json::from_str(&self.dependencies) {
                Ok(result) => result,
                Err(error) => return Err(format!("Could not parse dependencies:\n{error}")),
            },
        })
    }
}
