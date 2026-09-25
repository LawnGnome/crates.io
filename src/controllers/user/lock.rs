use axum::{
    Json,
    extract::{FromRequest, Path},
};
use axum_extra::{TypedHeader, headers::CacheControl};
use chrono::{DateTime, Utc};
use crates_io_api_types::EncodableUserLock;
use crates_io_database::{
    models::{User, users_by_username},
    schema::{oauth_github, users},
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::{StatusCode, request::Parts};
use serde::{Deserialize, Serialize};

use crate::{
    ServerContext,
    auth::AuthCheck,
    util::{
        errors::{AppResult, conflict},
        no_store,
    },
};

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UserLockGetResponse {
    pub lock: Option<EncodableUserLock>,
}

/// Get the lock status of the given user.
#[utoipa::path(
    get,
    path = "/api/v1/users/{user}/lock",
    params(
        ("user" = String, Path, description = "crates.io username"),
    ),
    security(("cookie" = [])),
    tags = ["users", "admin"],
    extensions(("x-internal" = json!(true))),
    responses(
        (status = 200, description = "Successful Response", body = inline(UserLockGetResponse)),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn get(
    ctx: ServerContext,
    Path(user_name): Path<String>,
    req: Parts,
) -> AppResult<(TypedHeader<CacheControl>, Json<UserLockGetResponse>)> {
    let mut conn = ctx.db_read_prefer_primary().await?;

    AuthCheck::only_cookie()
        .require_admin()
        .check(&req, &mut conn)
        .await?;

    let user = users_by_username(&user_name)
        .left_join(oauth_github::table)
        .select(User::as_select())
        .first(&mut conn)
        .await?;

    Ok((
        no_store(),
        Json(UserLockGetResponse {
            lock: user.is_locked().map(|(reason, until)| EncodableUserLock {
                reason: reason.into(),
                until,
            }),
        }),
    ))
}

/// Unlocks the given user.
#[utoipa::path(
    delete,
    path = "/api/v1/users/{user}/lock",
    params(
        ("user" = String, Path, description = "crates.io username"),
    ),
    security(("cookie" = [])),
    tags = ["users", "admin"],
    extensions(("x-internal" = json!(true))),
    responses(
        (status = 204, description = "Successful Response"),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn delete(
    ctx: ServerContext,
    Path(user_name): Path<String>,
    req: Parts,
) -> AppResult<StatusCode> {
    let mut conn = ctx.db_read_prefer_primary().await?;

    AuthCheck::only_cookie()
        .require_admin()
        .check(&req, &mut conn)
        .await?;

    let user = users_by_username(&user_name)
        .left_join(oauth_github::table)
        .select(User::as_select())
        .first(&mut conn)
        .await?;

    if user.account_lock_reason.is_none() {
        return Err(conflict("user is not currently locked"));
    }

    diesel::update(users::table)
        .set(users::account_lock_reason.eq(None::<String>))
        .filter(users::id.eq(user.id))
        .execute(&mut conn)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize, Serialize, FromRequest, utoipa::ToSchema)]
#[from_request(via(Json))]
pub struct PutRequest {
    pub reason: String,
    pub until: Option<DateTime<Utc>>,
}

/// Locks the given user.
#[utoipa::path(
    put,
    path = "/api/v1/users/{user}/lock",
    params(
        ("user" = String, Path, description = "crates.io username"),
    ),
    request_body = inline(PutRequest),
    security(("cookie" = [])),
    tags = ["users", "admin"],
    extensions(("x-internal" = json!(true))),
    responses(
        (status = 204, description = "Successful Response"),
        (status = "4XX", description = "Client Error", body = crate::util::errors::ApiErrorResponse<'_>),
        (status = "5XX", description = "Server Error", body = crate::util::errors::ApiErrorResponse<'_>),
    ),
)]
pub async fn put(
    ctx: ServerContext,
    Path(user_name): Path<String>,
    req: Parts,
    PutRequest { reason, until }: PutRequest,
) -> AppResult<StatusCode> {
    let mut conn = ctx.db_read_prefer_primary().await?;

    AuthCheck::only_cookie()
        .require_admin()
        .check(&req, &mut conn)
        .await?;

    let user = users_by_username(&user_name)
        .left_join(oauth_github::table)
        .select(User::as_select())
        .first(&mut conn)
        .await?;

    if user.account_lock_reason.is_some() {
        return Err(conflict("user is already locked"));
    }

    diesel::update(users::table)
        .set((
            users::account_lock_reason.eq(Some(reason)),
            users::account_lock_until.eq(until),
        ))
        .filter(users::id.eq(user.id))
        .execute(&mut conn)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
