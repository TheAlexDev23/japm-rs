use diesel::result::Error as QueryError;
use std::fmt;
use std::fmt::Display;

/// Error for performing any package db query that involves
/// json serialization/deserialization at any point
#[derive(Debug)]
pub enum TranslatedPackageQueryError {
    Query(QueryError),
    Json(serde_json::Error),
}
impl From<QueryError> for TranslatedPackageQueryError {
    fn from(other: QueryError) -> Self {
        TranslatedPackageQueryError::Query(other)
    }
}
impl From<serde_json::Error> for TranslatedPackageQueryError {
    fn from(other: serde_json::Error) -> Self {
        TranslatedPackageQueryError::Json(other)
    }
}

impl Display for TranslatedPackageQueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TranslatedPackageQueryError::Query(error) => write!(f, "{error}"),
            TranslatedPackageQueryError::Json(error) => write!(f, "{error}"),
        }
    }
}
