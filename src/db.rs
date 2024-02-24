use std::fmt::Display;
use std::fs::File;
use std::io;
use std::path::Path;

use tokio::fs;

use log::{info, trace};

use crate::package::{LocalPackage, PackageData, RemotePackage};

use diesel::prelude::*;

pub use errors::*;

// diesel has questionable naming
use diesel::result::{ConnectionError, Error as QueryError};

mod errors;

pub trait PackagesDb {
    type AddError: Display;
    type RemoveError: Display;
    type GetError: Display;

    fn add_package(&mut self, package: &RemotePackage) -> Result<(), Self::AddError>;
    fn remove_package(&mut self, package_name: &str) -> Result<(), Self::RemoveError>;
    fn get_package(&mut self, package_name: &str) -> Result<Option<LocalPackage>, Self::GetError>;
    fn get_all_packages(&mut self) -> Result<Vec<LocalPackage>, Self::GetError>;
    fn get_depending_packages(
        &mut self,
        package_name: &str,
    ) -> Result<Vec<LocalPackage>, Self::GetError>;
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
        pre_remove -> Text,
        package_files -> Text,
        post_remove -> Text,
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
    ///  Json array of pre_remove instructions
    pre_remove: String,
    ///  Json array of package filenames
    package_files: String,
    ///  Json array of post_remove instructions
    post_remove: String,
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
    ///  Json array of pre_remove instructions
    pub pre_remove: String,
    ///  Json array of package filenames
    pub package_files: String,
    ///  Json array of post_remove instructions
    pub post_remove: String,
    /// Json array of dependencies' names
    pub dependencies: String,
}

const DATABASE_SOURCE: &str = "/var/lib/japm/packages.db";
impl SqlitePackagesDb {
    pub fn new() -> Result<SqlitePackagesDb, ConnectionError> {
        let mut url = String::from("sqlite://");
        url.push_str(DATABASE_SOURCE);

        trace!("Establishing SQL connection with source:\n{url}");

        let connection = SqliteConnection::establish(&url)?;

        Ok(SqlitePackagesDb { connection })
    }

    pub async fn create_db_file_if_necessary() -> Result<bool, io::Error> {
        trace!("Creating db file if necessary");

        let database_path = Path::new(DATABASE_SOURCE);
        match database_path.try_exists()? {
            true => Ok(false),
            false => {
                info!("Database does not exist, creating...");

                trace!("Creating database parent directory");

                // Hardcoded directory allways has parent, unwrap is ok
                fs::create_dir_all(database_path.parent().unwrap()).await?;

                trace!("Creating database file");
                File::create(DATABASE_SOURCE)?;

                Ok(true)
            }
        }
    }

    pub fn initialize_database(&mut self) -> Result<(), QueryError> {
        const CREATE_TABLE_QUERY: &str = "CREATE TABLE packages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                description TEXT,
                pre_remove TEXT,
                package_files TEXT,
                post_remove TEXT,
                dependencies TEXT
            )";

        trace!("Executing SQL create table query:\n{CREATE_TABLE_QUERY}");

        diesel::sql_query(CREATE_TABLE_QUERY).execute(&mut self.connection)?;

        Ok(())
    }
}

impl PackagesDb for SqlitePackagesDb {
    type AddError = TranslatedPackageQueryError;
    type GetError = TranslatedPackageQueryError;
    type RemoveError = QueryError;

    fn add_package(&mut self, package: &RemotePackage) -> Result<(), TranslatedPackageQueryError> {
        use self::packages::dsl::*;

        let db_package: AddPackage = package.try_into()?;

        trace!("Inserting {db_package:#?} into the database");

        diesel::insert_into(packages)
            .values(db_package)
            .execute(&mut self.connection)?;

        Ok(())
    }

    fn remove_package(&mut self, package_name: &str) -> Result<(), QueryError> {
        use self::packages::dsl::*;

        diesel::delete(packages.filter(name.eq(package_name))).execute(&mut self.connection)?;

        Ok(())
    }

    fn get_package(
        &mut self,
        package_name: &str,
    ) -> Result<Option<LocalPackage>, TranslatedPackageQueryError> {
        use self::packages::dsl::*;

        match packages
            .filter(name.eq(package_name))
            .first::<GetPackage>(&mut self.connection)
            .optional()?
        {
            Some(package) => Ok(Some(<GetPackage as TryInto<LocalPackage>>::try_into(
                package,
            )?)),
            None => Ok(None),
        }
    }

    fn get_all_packages(&mut self) -> Result<Vec<LocalPackage>, TranslatedPackageQueryError> {
        use self::packages::dsl::*;

        let all_packages = packages
            .select(packages::all_columns())
            .load::<GetPackage>(&mut self.connection)?;

        let convert_into = |item: GetPackage| -> Result<LocalPackage, TranslatedPackageQueryError> {
            match item.try_into() {
                Ok(package) => Ok(package),
                Err(error) => Err(TranslatedPackageQueryError::Json(error)),
            }
        };

        all_packages.into_iter().map(convert_into).collect()
    }

    fn get_depending_packages(
        &mut self,
        package_name: &str,
    ) -> Result<Vec<LocalPackage>, TranslatedPackageQueryError> {
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
    type Error = serde_json::Error;

    fn try_from(package: &RemotePackage) -> Result<Self, Self::Error> {
        Ok(AddPackage {
            name: package.package_data.name.clone(),
            version: package.package_data.version.clone(),
            description: package.package_data.description.clone(),
            pre_remove: serde_json::to_string(&package.pre_remove)?,
            package_files: serde_json::to_string(&package.package_files)?,
            post_remove: serde_json::to_string(&package.post_remove)?,
            dependencies: serde_json::to_string(&package.dependencies)?,
        })
    }
}

impl TryInto<LocalPackage> for GetPackage {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<LocalPackage, Self::Error> {
        Ok(LocalPackage {
            package_data: PackageData {
                name: self.name,
                version: self.version,
                description: self.description,
            },
            pre_remove: serde_json::from_str(&self.pre_remove)?,
            package_files: serde_json::from_str(&self.package_files)?,
            post_remove: serde_json::from_str(&self.post_remove)?,
            dependencies: serde_json::from_str(&self.dependencies)?,
        })
    }
}
