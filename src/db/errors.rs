use thiserror::Error;

use diesel::result::Error as QueryError;

/// Error for performing any package db query that involves
/// json serialization/deserialization at any point
#[derive(Error, Debug)]
pub enum TranslatedPackageQueryError {
    #[error("A query error has occured: {0}")]
    Query(#[from] QueryError),
    #[error("A json serialization error has occured: {0}")]
    Json(#[from] serde_json::Error),
}
