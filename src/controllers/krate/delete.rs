use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::models::{Crate, Rights};
use crate::schema::{crate_downloads, crates, dependencies};
use crate::util::errors::{crate_not_found, custom, AppResult, BoxedAppError};
use crate::worker::jobs;
use axum::extract::Path;
use bigdecimal::ToPrimitive;
use chrono::Utc;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use http::request::Parts;
use http::StatusCode;

pub async fn delete(Path(name): Path<String>, parts: Parts, app: AppState) -> AppResult<()> {
    let mut conn = app.db_write().await?;

    // Check that the user is authenticated
    let auth = AuthCheck::only_cookie().check(&parts, &mut conn).await?;

    // Check that the crate exists
    let krate: Crate = Crate::by_name(&name)
        .first(&mut conn)
        .await
        .optional()?
        .ok_or_else(|| crate_not_found(&name))?;

    // Check that the user is an owner of the crate (team owners are not allowed to delete crates)
    let user = auth.user();
    let owners = krate.async_owners(&mut conn).await?;
    match user.rights(&app, &owners).await? {
        Rights::Full => {}
        Rights::Publish => {
            let msg = "team members don't have permission to delete crates";
            return Err(custom(StatusCode::FORBIDDEN, msg));
        }
        Rights::None => {
            let msg = "only owners have permission to delete crates";
            return Err(custom(StatusCode::FORBIDDEN, msg));
        }
    }

    // Check that the requirements for deleting the crate are met
    //
    // - The crate has been published for less than 72 hours,
    // - or if all the following conditions are met:
    //     - The crate has a single owner,
    //     - The crate has been downloaded less than 100 times for each month it has been published.
    //     - The crate is not depended upon by any other crate on crates.io (i.e. it has no reverse dependencies),

    let created_at = krate.created_at.and_utc();

    let is_old = created_at <= Utc::now() - chrono::Duration::hours(72);
    if is_old {
        if owners.len() > 1 {
            let msg = "only crates with a single owner can be deleted after 72 hours";
            return Err(custom(StatusCode::UNPROCESSABLE_ENTITY, msg));
        }

        const DOWNLOADS_PER_MONTH_LIMIT: u64 = 100;

        let age = Utc::now().signed_duration_since(created_at);
        let age_days = age.num_days().to_u64().unwrap_or(u64::MAX);
        let age_months = age_days.div_ceil(30);
        let max_downloads = DOWNLOADS_PER_MONTH_LIMIT * age_months;

        let downloads = crate_downloads::table
            .find(krate.id)
            .select(crate_downloads::downloads)
            .first::<i64>(&mut conn)
            .await
            .optional()?
            .unwrap_or_default()
            .to_u64()
            .unwrap_or(u64::MAX);

        if downloads > max_downloads {
            let msg =
                "only crates with less than 100 downloads per month can be deleted after 72 hours";
            return Err(custom(StatusCode::UNPROCESSABLE_ENTITY, msg));
        }

        let has_rev_dep = dependencies::table
            .filter(dependencies::crate_id.eq(&krate.id))
            .select(dependencies::id)
            .first::<i32>(&mut conn)
            .await
            .optional()?
            .is_some();

        if has_rev_dep {
            let msg = "only crates without reverse dependencies can be deleted after 72 hours";
            return Err(custom(StatusCode::UNPROCESSABLE_ENTITY, msg));
        }
    }

    conn.transaction(|conn| {
        async move {
            // Delete the crate
            diesel::delete(crates::table.find(krate.id))
                .execute(conn)
                .await?;

            // Enqueue index sync background jobs
            jobs::SyncToGitIndex::new(&krate.name)
                .async_enqueue(conn)
                .await?;

            jobs::SyncToSparseIndex::new(&krate.name)
                .async_enqueue(conn)
                .await?;

            // Enqueue deletion of corresponding files from S3
            jobs::DeleteCrateFromStorage::new(name)
                .async_enqueue(conn)
                .await?;

            Ok::<_, BoxedAppError>(())
        }
        .scope_boxed()
    })
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::OwnerKind;
    use crate::tests::builders::{DependencyBuilder, PublishBuilder};
    use crate::tests::util::{RequestHelper, Response, TestApp};
    use crates_io_database::schema::crate_owners;
    use diesel_async::AsyncPgConnection;
    use http::StatusCode;
    use insta::assert_snapshot;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_happy_path_new_crate() -> anyhow::Result<()> {
        let (app, anon, user) = TestApp::full().with_user();
        let mut conn = app.async_db_conn().await;
        let upstream = app.upstream_index();

        publish_crate(&user, "foo").await;
        let crate_id = adjust_creation_date(&mut conn, "foo", 71).await?;

        // Update downloads count so that it wouldn't be deletable if it wasn't new
        adjust_downloads(&mut conn, crate_id, 10_000).await?;

        assert_crate_exists(&anon, "foo", true).await;
        assert!(upstream.crate_exists("foo")?);
        assert_snapshot!(app.stored_files().await.join("\n"), @r"
        crates/foo/foo-1.0.0.crate
        index/3/f/foo
        rss/crates.xml
        rss/crates/foo.xml
        rss/updates.xml
        ");

        let response = delete_crate(&user, "foo").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.body().is_empty());

        // Assert that the crate no longer exists
        assert_crate_exists(&anon, "foo", false).await;
        assert!(!upstream.crate_exists("foo")?);
        assert_snapshot!(app.stored_files().await.join("\n"), @r"
        rss/crates.xml
        rss/updates.xml
        ");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_happy_path_old_crate() -> anyhow::Result<()> {
        let (app, anon, user) = TestApp::full().with_user();
        let mut conn = app.async_db_conn().await;
        let upstream = app.upstream_index();

        publish_crate(&user, "foo").await;
        let crate_id = adjust_creation_date(&mut conn, "foo", 73).await?;
        adjust_downloads(&mut conn, crate_id, 100).await?;

        assert_crate_exists(&anon, "foo", true).await;
        assert!(upstream.crate_exists("foo")?);
        assert_snapshot!(app.stored_files().await.join("\n"), @r"
        crates/foo/foo-1.0.0.crate
        index/3/f/foo
        rss/crates.xml
        rss/crates/foo.xml
        rss/updates.xml
        ");

        let response = delete_crate(&user, "foo").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.body().is_empty());

        // Assert that the crate no longer exists
        assert_crate_exists(&anon, "foo", false).await;
        assert!(!upstream.crate_exists("foo")?);
        assert_snapshot!(app.stored_files().await.join("\n"), @r"
        rss/crates.xml
        rss/updates.xml
        ");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_no_auth() -> anyhow::Result<()> {
        let (_app, anon, user) = TestApp::full().with_user();

        publish_crate(&user, "foo").await;

        let response = delete_crate(&anon, "foo").await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);

        assert_crate_exists(&anon, "foo", true).await;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_token_auth() -> anyhow::Result<()> {
        let (_app, anon, user, token) = TestApp::full().with_token();

        publish_crate(&user, "foo").await;

        let response = delete_crate(&token, "foo").await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action can only be performed on the crates.io website"}]}"#);

        assert_crate_exists(&anon, "foo", true).await;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_missing_crate() -> anyhow::Result<()> {
        let (_app, _anon, user) = TestApp::full().with_user();

        let response = delete_crate(&user, "foo").await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate `foo` does not exist"}]}"#);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_not_owner() -> anyhow::Result<()> {
        let (app, anon, user) = TestApp::full().with_user();
        let user2 = app.db_new_user("bar");

        publish_crate(&user, "foo").await;

        let response = delete_crate(&user2, "foo").await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"only owners have permission to delete crates"}]}"#);

        assert_crate_exists(&anon, "foo", true).await;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_team_owner() -> anyhow::Result<()> {
        let (app, anon) = TestApp::full().empty();
        let user = app.db_new_user("user-org-owner");
        let user2 = app.db_new_user("user-one-team");

        publish_crate(&user, "foo").await;

        // Add team owner
        let body = json!({ "owners": ["github:test-org:all"] }).to_string();
        let response = user.put::<()>("/api/v1/crates/foo/owners", body).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = delete_crate(&user2, "foo").await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"team members don't have permission to delete crates"}]}"#);

        assert_crate_exists(&anon, "foo", true).await;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_too_many_owners() -> anyhow::Result<()> {
        let (app, anon, user) = TestApp::full().with_user();
        let mut conn = app.async_db_conn().await;
        let user2 = app.db_new_user("bar");

        publish_crate(&user, "foo").await;
        let crate_id = adjust_creation_date(&mut conn, "foo", 73).await?;

        // Add another owner
        diesel::insert_into(crate_owners::table)
            .values((
                crate_owners::crate_id.eq(crate_id),
                crate_owners::owner_id.eq(user2.as_model().id),
                crate_owners::owner_kind.eq(OwnerKind::User),
            ))
            .execute(&mut conn)
            .await?;

        let response = delete_crate(&user, "foo").await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"only crates with a single owner can be deleted after 72 hours"}]}"#);

        assert_crate_exists(&anon, "foo", true).await;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_too_many_downloads() -> anyhow::Result<()> {
        let (app, anon, user) = TestApp::full().with_user();
        let mut conn = app.async_db_conn().await;

        publish_crate(&user, "foo").await;
        let crate_id = adjust_creation_date(&mut conn, "foo", 73).await?;
        adjust_downloads(&mut conn, crate_id, 101).await?;

        let response = delete_crate(&user, "foo").await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"only crates with less than 100 downloads per month can be deleted after 72 hours"}]}"#);

        assert_crate_exists(&anon, "foo", true).await;

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_rev_deps() -> anyhow::Result<()> {
        let (app, anon, user) = TestApp::full().with_user();
        let mut conn = app.async_db_conn().await;

        publish_crate(&user, "foo").await;
        adjust_creation_date(&mut conn, "foo", 73).await?;

        // Publish another crate
        let pb = PublishBuilder::new("bar", "1.0.0").dependency(DependencyBuilder::new("foo"));
        let response = user.publish_crate(pb).await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = delete_crate(&user, "foo").await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"only crates without reverse dependencies can be deleted after 72 hours"}]}"#);

        assert_crate_exists(&anon, "foo", true).await;

        Ok(())
    }

    // Publishes a crate with the given name and a single `v1.0.0` version.
    async fn publish_crate(user: &impl RequestHelper, name: &str) {
        let pb = PublishBuilder::new(name, "1.0.0");
        let response = user.publish_crate(pb).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Moves the `created_at` field of a crate by the given number of hours
    /// into the past and returns the ID of the crate.
    async fn adjust_creation_date(
        conn: &mut AsyncPgConnection,
        name: &str,
        hours: i64,
    ) -> QueryResult<i32> {
        let created_at = Utc::now() - chrono::Duration::hours(hours);
        let created_at = created_at.naive_utc();

        diesel::update(crates::table)
            .filter(crates::name.eq(name))
            .set(crates::created_at.eq(created_at))
            .returning(crates::id)
            .get_result(conn)
            .await
    }

    // Updates the download count of a crate.
    async fn adjust_downloads(
        conn: &mut AsyncPgConnection,
        crate_id: i32,
        downloads: i64,
    ) -> QueryResult<()> {
        diesel::update(crate_downloads::table)
            .filter(crate_downloads::crate_id.eq(crate_id))
            .set(crate_downloads::downloads.eq(downloads))
            .execute(conn)
            .await?;

        Ok(())
    }

    // Performs the `DELETE` request to delete the crate, and runs any pending
    // background jobs, then returns the response.
    async fn delete_crate(user: &impl RequestHelper, name: &str) -> Response<()> {
        let url = format!("/api/v1/crates/{name}");
        let response = user.delete::<()>(&url).await;
        user.app().run_pending_background_jobs().await;
        response
    }

    // Asserts that the crate with the given name exists or not.
    async fn assert_crate_exists(user: &impl RequestHelper, name: &str, exists: bool) {
        let expected_status = match exists {
            true => StatusCode::OK,
            false => StatusCode::NOT_FOUND,
        };

        let url = format!("/api/v1/crates/{name}");
        let response = user.get::<()>(&url).await;
        assert_eq!(response.status(), expected_status);
    }
}
