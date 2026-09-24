use claims::assert_none;
use crates_io::{controllers::user::lock::UserLockGetResponse, models::User, schema::users};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use http::StatusCode;
use insta::assert_json_snapshot;

use crate::util::{RequestHelper, TestApp};

#[tokio::test(flavor = "multi_thread")]
async fn get_anon() {
    let (_, anon, user) = TestApp::init().with_user().await;
    let url = format!("/api/v1/users/{}/lock", user.as_model().username);

    // Anonymous users should not have access.
    let response = anon.get::<()>(&url).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread")]
async fn get_regular_user() {
    let (_, _, user) = TestApp::init().with_user().await;
    let url = format!("/api/v1/users/{}/lock", user.as_model().username);

    // Normal users should not have access.
    let response = user.get::<()>(&url).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread")]
async fn get_admin_unlocked() {
    let (app, _, user) = TestApp::init().with_user().await;
    let url = format!("/api/v1/users/{}/lock", user.as_model().username);

    // Let's create an admin.
    let admin = app.db_new_admin_user("admin").await;

    // Now we should be able to see that the user is not currently locked.
    let response = admin.get::<UserLockGetResponse>(&url).await.good();
    assert_none!(response.lock);
}

#[tokio::test(flavor = "multi_thread")]
async fn get_admin_locked() -> anyhow::Result<()> {
    let (app, _, user) = TestApp::init().with_user().await;
    let conn = app.db_conn().await;
    let url = format!("/api/v1/users/{}/lock", user.as_model().username);

    // Let's create an admin.
    let admin = app.db_new_admin_user("admin").await;

    // Let's lock the user.
    lock_user(&conn, user.as_model()).await?;

    let response = admin.get::<UserLockGetResponse>(&url).await.good();
    assert_json_snapshot!(response, @r#"
    {
      "lock": {
        "reason": "test",
        "until": null
      }
    }
    "#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_anon() {
    let (_, anon, user) = TestApp::init().with_user().await;
    let url = format!("/api/v1/users/{}/lock", user.as_model().username);

    // Anonymous users should not have access.
    let response = anon.delete::<()>(&url).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_regular_user() {
    let (_, _, user) = TestApp::init().with_user().await;
    let url = format!("/api/v1/users/{}/lock", user.as_model().username);

    // Normal users should not have access.
    let response = user.delete::<()>(&url).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_admin_unlocked() {
    let (app, _, user) = TestApp::init().with_user().await;
    let url = format!("/api/v1/users/{}/lock", user.as_model().username);

    // Let's create an admin.
    let admin = app.db_new_admin_user("admin").await;

    // This request will fail, since the user isn't currently locked.
    let response = admin.delete::<()>(&url).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_admin_locked() -> anyhow::Result<()> {
    let (app, _, user) = TestApp::init().with_user().await;
    let conn = app.db_conn().await;
    let url = format!("/api/v1/users/{}/lock", user.as_model().username);

    // Let's create an admin.
    let admin = app.db_new_admin_user("admin").await;

    // Let's lock the user.
    lock_user(&conn, user.as_model()).await?;

    let response = admin.delete::<()>(&url).await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // And let's check that the user really was unlocked.
    let user = User::find(&conn, user.as_model().id).await?;
    assert_none!(user.is_locked());

    Ok(())
}

async fn lock_user(mut conn: &AsyncPgConnection, user: &User) -> anyhow::Result<()> {
    diesel::update(users::table)
        .set(users::account_lock_reason.eq(Some("test")))
        .filter(users::id.eq(user.id))
        .execute(&mut conn)
        .await?;

    Ok(())
}
