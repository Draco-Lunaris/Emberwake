//! Typed application error with ServerFnError mapping.
//! Carries typed validation and authorization failures surfaced inline in the UI.
//! AppError implements Display + FromStr so it can be used as the CustErr type
//! parameter for ServerFnError<AppError>.

use std::str::FromStr;

use thiserror::Error;

/// Typed application error — the custom error type for server functions.
#[derive(Debug, Clone, Error)]
pub enum AppError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("authentication required")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("rate limited")]
    RateLimited,
    #[error("internal error")]
    Internal,
}

impl AppError {
    pub fn status_code(&self) -> u16 {
        match self {
            Self::Validation(_) => 400,
            Self::Unauthorized => 401,
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::Conflict(_) => 409,
            Self::RateLimited => 429,
            Self::Internal => 500,
        }
    }
}

impl FromStr for AppError {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(rest) = s.strip_prefix("validation error: ") {
            Ok(Self::Validation(rest.to_string()))
        } else if s == "authentication required" {
            Ok(Self::Unauthorized)
        } else if s == "forbidden" {
            Ok(Self::Forbidden)
        } else if s == "not found" {
            Ok(Self::NotFound)
        } else if let Some(rest) = s.strip_prefix("conflict: ") {
            Ok(Self::Conflict(rest.to_string()))
        } else if s == "rate limited" {
            Ok(Self::RateLimited)
        } else {
            Ok(Self::Internal)
        }
    }
}

#[cfg(feature = "ssr")]
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::NotFound,
            sqlx::Error::Database(ref db) => {
                if db.is_unique_violation() {
                    Self::Conflict("duplicate value".into())
                } else {
                    Self::Internal
                }
            }
            _ => Self::Internal,
        }
    }
}

impl From<garde::Report> for AppError {
    fn from(report: garde::Report) -> Self {
        let msgs: Vec<String> = report
            .iter()
            .map(|(path, error)| format!("{path}: {error}"))
            .collect();
        Self::Validation(msgs.join("; "))
    }
}
