#![allow(unused)]
use actix_web::{
    body,
    http::{header, StatusCode},
    HttpResponse, ResponseError,
};
use deadpool_redis::{redis::RedisError, CreatePoolError, PoolError};
use serde_json::json;
use std::borrow::Cow;

use crate::ENV;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Bad Request: {0}")]
    BadRequest(Cow<'static, str>),
    #[error("Unauthorized: {0}")]
    Unauthorized(Cow<'static, str>),
    #[error("Forbidden: {0}")]
    Forbidden(Cow<'static, str>),
    #[error("Not Found: {0}")]
    NotFound(Cow<'static, str>),
    #[error("Conflict: {0}")]
    Conflict(Cow<'static, str>),
    #[error("Internal Server Error")]
    InternalServer,
}

#[derive(serde::Serialize)]
pub struct ErrorBody {
    pub message: Cow<'static, str>,
}

impl Error {
    pub fn bad_request(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::BadRequest(msg.into())
    }

    pub fn unauthorized(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::Unauthorized(msg.into())
    }

    pub fn forbidden(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::Forbidden(msg.into())
    }

    pub fn not_found(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn conflict(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::Conflict(msg.into())
    }

    pub fn internal_server_error() -> Self {
        Self::InternalServer
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match *self {
            Error::BadRequest(_) => StatusCode::BAD_REQUEST,
            Error::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Error::Forbidden(_) => StatusCode::FORBIDDEN,
            Error::NotFound(_) => StatusCode::NOT_FOUND,
            Error::Conflict(_) => StatusCode::CONFLICT,
            Error::InternalServer => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let header = ("Access-Control-Allow-Origin", ENV.frontend_url.as_str());
        let mut res = HttpResponse::build(self.status_code());

        res.insert_header(header);
        res.insert_header(("Access-Control-Allow-Credentials", "true"));

        match self {
            // Has Message
            Error::NotFound(msg)
            | Error::Conflict(msg)
            | Error::Unauthorized(msg)
            | Error::BadRequest(msg)
            | Error::Forbidden(msg) => res.json(ErrorBody { message: msg.clone() }),
            // No Message
            Error::InternalServer => {
                res.json(ErrorBody { message: "Internal Server Error".into() })
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum SystemError {
    // jwt errors
    #[error("JWT Error")]
    JwtError(#[from] jsonwebtoken::errors::Error),
    // argon2 errors
    #[error("Hash Error")]
    HashError(#[from] argon2::password_hash::Error),
    // sqlx errors
    #[error("Database Error : {0}")]
    DatabaseError(Cow<'static, str>),
    // serde errors
    #[error("JSON Serialization/Deserialization Error")]
    JsonError(#[from] serde_json::Error),
    // redis errors
    #[error(transparent)]
    PoolInit(#[from] CreatePoolError),
    #[error("Redis pool error: {0}")]
    PoolGet(#[from] PoolError),
    #[error("Redis error")]
    RedisError(#[from] RedisError),
    // Custom Errors
    #[error("Bad Request: {0}")]
    BadRequest(Cow<'static, str>),
    #[error("Unauthorized: {0}")]
    Unauthorized(Cow<'static, str>),
    #[error("Forbidden: {0}")]
    Forbidden(Cow<'static, str>),
    #[error("Database Not Found: {0}")]
    NotFound(Cow<'static, str>),
    #[error("Database Conflict: {0:?}")]
    Conflict(Option<DbErrorMeta>),
    #[error("Internal System Error: {0}")]
    InternalError(Box<dyn std::error::Error + Send + Sync>),
}

fn conflict_message(meta: &Option<DbErrorMeta>) -> Cow<'static, str> {
    let Some(m) = meta else {
        return "Duplicate value".into();
    };

    let Some(constraint) = &m.constraint else {
        return "Duplicate value".into();
    };

    let field = constraint.split('_').next_back().unwrap_or("value");

    let mut chars = field.chars();
    let field = match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => "Value".to_string(),
    };

    format!("{field} already exists").into()
}

#[derive(Debug)]
pub struct DbErrorMeta {
    pub code: Option<String>,
    pub constraint: Option<String>,
    pub message: String,
}

impl From<SystemError> for Error {
    fn from(value: SystemError) -> Self {
        match value {
            SystemError::BadRequest(msg) => Error::BadRequest(msg),
            SystemError::Unauthorized(msg) => Error::Unauthorized(msg),
            SystemError::Forbidden(msg) => Error::Forbidden(msg),
            SystemError::NotFound(msg) => Error::NotFound(msg),
            SystemError::Conflict(meta) => Error::Conflict(conflict_message(&meta)),
            _ => {
                log::error!("Internal Server Error: {:?}", value);
                Error::InternalServer
            }
        }
    }
}

impl From<sqlx::Error> for SystemError {
    fn from(err: sqlx::Error) -> Self {
        log::error!("{:?}", err);
        if let sqlx::Error::Database(db_err) = &err {
            match db_err.code().as_deref() {
                Some("23505") => {
                    return SystemError::Conflict(Some(DbErrorMeta {
                        code: db_err.code().map(|s| s.to_string()),
                        constraint: db_err.constraint().map(|s| s.to_string()),
                        message: db_err.message().to_string(),
                    }));
                }
                Some("42P01") => {
                    return SystemError::NotFound("Resource not found".into());
                }
                _ => {
                    log::error!("Unhandled DB error: {:?}", db_err);
                    return SystemError::DatabaseError(db_err.message().to_string().into());
                }
            }
        }
        (SystemError::InternalError(Box::new(err)))
    }
}

impl SystemError {
    pub fn bad_request(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::BadRequest(msg.into())
    }

    pub fn not_found(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn unauthorized(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::Unauthorized(msg.into())
    }

    pub fn forbidden(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::Forbidden(msg.into())
    }
}
