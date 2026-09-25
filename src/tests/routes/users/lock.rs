use claims::{assert_none, assert_some};
use crates_io::{
    controllers::user::lock::{PutRequest, UserLockGetResponse},
    models::User,
    schema::users,
};
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

#[tokio::test(flavor = "multi_thread")]
async fn put_anon() -> anyhow::Result<()> {
    let (_, anon, user) = TestApp::init().with_user().await;
    let url = format!("/api/v1/users/{}/lock", user.as_model().username);

    // Anonymous users should not have access.
    let response = anon.put::<()>(&url, put_body()?).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn put_regular_user() -> anyhow::Result<()> {
    let (_, _, user) = TestApp::init().with_user().await;
    let url = format!("/api/v1/users/{}/lock", user.as_model().username);

    // Normal users should not have access.
    let response = user.put::<()>(&url, put_body()?).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn put_admin_unlocked() -> anyhow::Result<()> {
    let (app, _, user) = TestApp::init().with_user().await;
    let conn = app.db_conn().await;
    let url = format!("/api/v1/users/{}/lock", user.as_model().username);

    // Let's create an admin.
    let admin = app.db_new_admin_user("admin").await;

    // This will succeed, since the user is currently unlocked.
    let response = admin.put::<()>(&url, put_body()?).await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Check that the user really was locked.
    let user = User::find(&conn, user.as_model().id).await?;
    let (reason, until) = assert_some!(user.is_locked());
    assert_eq!(reason, "test");
    assert_none!(until);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn put_admin_locked() -> anyhow::Result<()> {
    let (app, _, user) = TestApp::init().with_user().await;
    let conn = app.db_conn().await;
    let url = format!("/api/v1/users/{}/lock", user.as_model().username);

    // Let's create an admin.
    let admin = app.db_new_admin_user("admin").await;

    // Let's lock the user.
    lock_user(&conn, user.as_model()).await?;

    // We should now get an error, since the user is already locked.
    let response = admin.put::<()>(&url, put_body()?).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);

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

fn put_body() -> anyhow::Result<Vec<u8>> {
    Ok(serde_json::to_vec(&PutRequest {
        reason: "test".into(),
        until: None,
    })?)
}
