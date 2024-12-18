use axum::{extract::Path, Json};
use chrono::{NaiveDateTime, Utc};
use crates_io_database::schema::{emails, users};
use diesel::{pg::Pg, prelude::*};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection, RunQueryDsl};
use http::request::Parts;

use crate::{
    app::AppState,
    auth::AuthCheck,
    models::User,
    sql::lower,
    util::{errors::AppResult, rfc3339},
    views::EncodableAdminUser,
};

/// Handles the `GET /users/:user_id/admin` route.
///
/// This returns the admin's eye view of a user, including account locking
/// metadata.
pub async fn get(
    state: AppState,
    Path(user_name): Path<String>,
    req: Parts,
) -> AppResult<Json<EncodableAdminUser>> {
    let mut conn = state.db_read_prefer_primary().await?;
    AuthCheck::only_cookie()
        .require_admin()
        .check(&req, &mut conn)
        .await?;

    get_user(
        |query| query.filter(lower(users::gh_login).eq(lower(user_name))),
        &mut conn,
    )
    .await
    .map(Json)
}

#[derive(Deserialize)]
pub struct LockRequest {
    reason: String,
    #[serde(default, with = "rfc3339::option")]
    until: Option<NaiveDateTime>,
}

/// Handles the `PUT /users/:user_id/lock` route.
pub async fn lock(
    state: AppState,
    Path(user_name): Path<String>,
    req: Parts,
    Json(LockRequest { reason, until }): Json<LockRequest>,
) -> AppResult<Json<EncodableAdminUser>> {
    let mut conn = state.db_read_prefer_primary().await?;
    AuthCheck::only_cookie()
        .require_admin()
        .check(&req, &mut conn)
        .await?;

    // In theory, we could cook up a complicated update query that returns
    // everything we need to build an `EncodableAdminUser`, but that feels hard.
    // Instead, let's use a small transaction to get the same effect.
    let user = conn
        .transaction(|conn| {
            async move {
                let id = diesel::update(users::table)
                    .filter(lower(users::gh_login).eq(lower(user_name)))
                    .set((
                        users::account_lock_reason.eq(reason),
                        users::account_lock_until.eq(until),
                    ))
                    .returning(users::id)
                    .get_result::<i32>(conn)
                    .await?;

                get_user(|query| query.filter(users::id.eq(id)), conn).await
            }
            .scope_boxed()
        })
        .await?;

    Ok(Json(user))
}

/// Handles the `DELETE /users/:user_id/lock` route.
pub async fn unlock(
    state: AppState,
    Path(user_name): Path<String>,
    req: Parts,
) -> AppResult<Json<EncodableAdminUser>> {
    let mut conn = state.db_read_prefer_primary().await?;
    AuthCheck::only_cookie()
        .require_admin()
        .check(&req, &mut conn)
        .await?;

    // Again, let's do this in a transaction, even though we _technically_ don't
    // need to.
    let user = conn
        .transaction(|conn| {
            // Although this is called via the `DELETE` method, this is
            // implemented as a soft deletion by setting the lock until time to
            // now, thereby allowing us to have some sense of history of whether
            // an account has been locked in the past.
            async move {
                let id = diesel::update(users::table)
                    .filter(lower(users::gh_login).eq(lower(user_name)))
                    .set(users::account_lock_until.eq(Utc::now().naive_utc()))
                    .returning(users::id)
                    .get_result::<i32>(conn)
                    .await?;

                get_user(|query| query.filter(users::id.eq(id)), conn).await
            }
            .scope_boxed()
        })
        .await?;

    Ok(Json(user))
}

/// A helper to get an [`EncodableAdminUser`] based on whatever filter predicate
/// is provided in the callback.
///
/// It would be ill advised to do anything in `filter` other than calling
/// [`QueryDsl::filter`] on the given query, but I'm not the boss of you.
async fn get_user<Conn, F>(filter: F, conn: &mut Conn) -> AppResult<EncodableAdminUser>
where
    Conn: AsyncConnection<Backend = Pg>,
    F: FnOnce(users::BoxedQuery<'_, Pg>) -> users::BoxedQuery<'_, Pg>,
{
    let query = filter(users::table.into_boxed());

    let (user, verified, email, verification_sent): (User, Option<bool>, Option<String>, bool) =
        query
            .left_join(emails::table)
            .select((
                User::as_select(),
                emails::verified.nullable(),
                emails::email.nullable(),
                emails::token_generated_at.nullable().is_not_null(),
            ))
            .first(conn)
            .await?;

    let verified = verified.unwrap_or(false);
    let verification_sent = verified || verification_sent;
    Ok(EncodableAdminUser::from(
        user,
        email,
        verified,
        verification_sent,
    ))
}
