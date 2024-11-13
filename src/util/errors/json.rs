use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use std::borrow::Cow;
use std::fmt;

use super::{AppError, BoxedAppError};

use crate::middleware::log_request::CauseField;
use crate::rate_limiter::LimitedAction;
use chrono::NaiveDateTime;
use http::{header, StatusCode};

/// Generates a response with the provided status and description as JSON
fn json_error(detail: &str, status: StatusCode) -> Response {
    let json = json!({ "errors": [{ "detail": detail }] });
    (status, Json(json)).into_response()
}

// The following structs are empty and do not provide a custom message to the user

#[derive(Debug)]
pub(crate) struct ReadOnlyMode;

impl AppError for ReadOnlyMode {
    fn response(&self) -> Response {
        let detail = "crates.io is currently in read-only mode for maintenance. \
                      Please try again later.";
        json_error(detail, StatusCode::SERVICE_UNAVAILABLE)
    }
}

impl fmt::Display for ReadOnlyMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "Tried to write in read only mode".fmt(f)
    }
}

// The following structs wrap owned data and provide a custom message to the user

pub fn custom(status: StatusCode, detail: impl Into<Cow<'static, str>>) -> BoxedAppError {
    Box::new(CustomApiError {
        status,
        detail: Detail::Single(detail.into()),
    })
}

#[derive(Debug, Clone)]
pub struct CustomApiError {
    status: StatusCode,
    detail: Detail,
}

#[derive(Debug, Clone)]
enum Detail {
    Empty,
    Single(Cow<'static, str>),
    Multiple(Vec<Cow<'static, str>>),
}

impl fmt::Display for Detail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, ""),
            Self::Single(msg) => write!(f, "{msg}"),
            Self::Multiple(msgs) => write!(f, "{}", msgs.join(", ")),
        }
    }
}

impl CustomApiError {
    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            detail: Detail::Empty,
        }
    }

    pub fn contains_errors(&self) -> bool {
        !self.is_empty()
    }

    pub fn is_empty(&self) -> bool {
        matches!(&self.detail, Detail::Empty)
    }

    pub fn push(&mut self, detail: impl Into<Cow<'static, str>>) -> &mut Self {
        match &mut self.detail {
            Detail::Empty => {
                self.detail = Detail::Single(detail.into());
            }
            Detail::Single(msg) => {
                let msg = msg.clone();
                self.detail = Detail::Multiple(vec![msg, detail.into()]);
            }
            Detail::Multiple(msgs) => {
                msgs.push(detail.into());
            }
        }

        self
    }
}

impl From<CustomApiError> for BoxedAppError {
    fn from(value: CustomApiError) -> Self {
        Box::new(value)
    }
}

impl fmt::Display for CustomApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.detail.fmt(f)
    }
}

impl AppError for CustomApiError {
    fn response(&self) -> Response {
        #[derive(Serialize)]
        struct ErrorContent {
            detail: Cow<'static, str>,
        }

        impl From<&Cow<'static, str>> for ErrorContent {
            fn from(value: &Cow<'static, str>) -> Self {
                Self {
                    detail: value.clone(),
                }
            }
        }

        #[derive(Serialize)]
        struct ErrorBody {
            errors: Vec<ErrorContent>,
        }

        let body = ErrorBody {
            errors: match &self.detail {
                Detail::Empty => Vec::new(),
                Detail::Single(msg) => vec![msg.into()],
                Detail::Multiple(msgs) => msgs.iter().map(|msg| msg.into()).collect(),
            },
        };

        (self.status, Json(body)).into_response()
    }
}

#[derive(Debug)]
pub(crate) struct TooManyRequests {
    pub action: LimitedAction,
    pub retry_after: NaiveDateTime,
}

impl AppError for TooManyRequests {
    fn response(&self) -> Response {
        const HTTP_DATE_FORMAT: &str = "%a, %d %b %Y %H:%M:%S GMT";
        let retry_after = self.retry_after.format(HTTP_DATE_FORMAT);

        let detail = format!(
            "{}. Please try again after {retry_after} or email \
             help@crates.io to have your limit increased.",
            self.action.error_message()
        );
        let mut response = json_error(&detail, StatusCode::TOO_MANY_REQUESTS);
        response.headers_mut().insert(
            header::RETRY_AFTER,
            retry_after
                .to_string()
                .try_into()
                .expect("HTTP_DATE_FORMAT contains invalid char"),
        );
        response
    }
}

impl fmt::Display for TooManyRequests {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "Too many requests".fmt(f)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InsecurelyGeneratedTokenRevoked;

impl InsecurelyGeneratedTokenRevoked {
    pub fn boxed() -> BoxedAppError {
        Box::new(Self)
    }
}

impl AppError for InsecurelyGeneratedTokenRevoked {
    fn response(&self) -> Response {
        let cause = CauseField("insecurely generated, revoked 2020-07".to_string());
        let response = json_error(&self.to_string(), StatusCode::UNAUTHORIZED);
        (Extension(cause), response).into_response()
    }
}

pub const TOKEN_FORMAT_ERROR: &str =
    "The given API token does not match the format used by crates.io. \
    \
    Tokens generated before 2020-07-14 were generated with an insecure \
    random number generator, and have been revoked. You can generate a \
    new token at https://crates.io/me. \
    \
    For more information please see \
    https://blog.rust-lang.org/2020/07/14/crates-io-security-advisory.html. \
    We apologize for any inconvenience.";

impl fmt::Display for InsecurelyGeneratedTokenRevoked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(TOKEN_FORMAT_ERROR)?;
        Result::Ok(())
    }
}
