use anyhow::Context;
use crates_io_database::utils::token::HashedToken;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{
    App,
    email::Email,
    models::{ApiToken, User},
    schema::api_tokens,
};

/// An exposed API token alert received from an upstream provider.
pub trait SecretAlert: Send + Sync + Sized {
    fn reporter(&self) -> &str;
    fn source(&self) -> Option<&str>;
    fn token(&self) -> &str;
    fn url(&self) -> Option<&str>;

    /// Revokes the given token, sending an e-mail to the user with details on which token was
    /// revoked and why.
    async fn revoke_token(
        &self,
        app: &App,
        conn: &mut AsyncPgConnection,
    ) -> QueryResult<FeedbackLabel> {
        let hashed_token = HashedToken::hash(self.token());

        // Not using `ApiToken::find_by_api_token()` in order to preserve `last_used_at`
        let token = api_tokens::table
            .select(ApiToken::as_select())
            .filter(api_tokens::token.eq(hashed_token))
            .get_result::<ApiToken>(conn)
            .await
            .optional()?;

        let Some(token) = token else {
            debug!("Unknown API token received (false positive)");
            return Ok(FeedbackLabel::FalsePositive);
        };

        if token.revoked {
            debug!(
                token_id = %token.id, user_id = %token.user_id,
                "Already revoked API token received (true positive)",
            );
            return Ok(FeedbackLabel::TruePositive);
        }

        diesel::update(&token)
            .set(api_tokens::revoked.eq(true))
            .execute(conn)
            .await?;

        warn!(
            token_id = %token.id, user_id = %token.user_id,
            "Active API token received and revoked (true positive)",
        );

        if let Err(error) = send_notification_email(&token, self, app, conn).await {
            warn!(
                token_id = %token.id, user_id = %token.user_id, ?error,
                "Failed to send email notification",
            )
        }

        Ok(FeedbackLabel::TruePositive)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackLabel {
    TruePositive,
    FalsePositive,
}

async fn send_notification_email<A>(
    token: &ApiToken,
    alert: &A,
    app: &App,
    conn: &mut AsyncPgConnection,
) -> anyhow::Result<()>
where
    A: SecretAlert,
{
    let user = User::find(conn, token.user_id)
        .await
        .context("Failed to find user")?;

    let Some(recipient) = user.email(conn).await? else {
        return Err(anyhow::anyhow!("No address found"));
    };

    let email = TokenExposedEmail {
        domain: &app.config.domain_name,
        reporter: alert.reporter(),
        source: alert.source(),
        token_name: &token.name,
        url: alert.url(),
    };

    app.emails.send(&recipient, email).await?;

    Ok(())
}

struct TokenExposedEmail<'a> {
    domain: &'a str,
    reporter: &'a str,
    source: Option<&'a str>,
    token_name: &'a str,
    url: Option<&'a str>,
}

impl Email for TokenExposedEmail<'_> {
    fn subject(&self) -> String {
        format!(
            "crates.io: Your API token \"{}\" has been revoked",
            self.token_name
        )
    }

    fn body(&self) -> String {
        let mut body = format!(
            "{reporter} has notified us that your crates.io API token {token_name} \
has been exposed publicly. We have revoked this token as a precaution.

Please review your account at https://{domain} to confirm that no \
unexpected changes have been made to your settings or crates.",
            domain = self.domain,
            reporter = self.reporter,
            token_name = self.token_name,
        );

        if let Some(source) = &self.source {
            body.push_str(&format!("\n\nSource type: {source}"));
        }

        if let Some(url) = &self.url {
            body.push_str(&format!("\n\nURL where the token was found: {url}"));
        } else {
            body.push_str("\n\nWe were not informed of the URL where the token was found.");
        }

        body
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Deref;

    use insta::assert_snapshot;
    use secrecy::ExposeSecret;

    use crate::tests::util::TestApp;

    use super::*;

    #[tokio::test]
    async fn test_revoke_token() -> anyhow::Result<()> {
        let (test_app, _, _, token) = TestApp::init().with_token().await;
        let mut conn = test_app.db_conn().await;
        let app = test_app.as_inner();

        let alert = MockAlert {
            reporter: "GitHub",
            source: Some("pull request"),
            token: token.plaintext().expose_secret().to_string(),
            url: Some("https://github.com/"),
        };

        // This should queue up an e-mail, revoke the token, and return that it's a true positive.
        assert_eq!(
            alert.revoke_token(app, &mut conn).await?,
            FeedbackLabel::TruePositive
        );

        // Check that an e-mail was queued.
        let mails = assert_some!(app.emails.mails_in_memory().await);
        assert_eq!(mails.len(), 1);

        // Check the body. Note that we have to do some redactions here for dynamic parts of the
        // body; insta's redaction feature is overkill for this, but we need something.
        let body = assert_some!(<Vec<_> as Deref>::deref(&mails).first())
            .1
            .split("\r\n")
            .filter(|line| !(line.starts_with("Message-ID: ") || line.starts_with("Date: ")))
            .collect::<Vec<_>>()
            .join("\r\n");
        assert_snapshot!("body", body);

        // Check that the token was actually revoked.
        let hashed = token.plaintext().hashed();
        let db_token = api_tokens::table
            .select(ApiToken::as_select())
            .filter(api_tokens::token.eq(&hashed))
            .get_result::<ApiToken>(&mut conn)
            .await?;
        assert_eq!(db_token.id, token.as_model().id);
        assert!(db_token.revoked);

        // Repeating the revocation should result in no new e-mail or action, but should still
        // return that it's a true positive.
        assert_eq!(
            alert.revoke_token(app, &mut conn).await?,
            FeedbackLabel::TruePositive
        );

        let mails = assert_some!(app.emails.mails_in_memory().await);
        assert_eq!(mails.len(), 1);

        // Trying to revoke a token that doesn't exist should, of course, result in a false
        // positive.
        assert_eq!(
            MockAlert {
                reporter: "GitHub",
                source: Some("pull request"),
                token: "not a valid token".to_string(),
                url: Some("https://github.com/"),
            }
            .revoke_token(app, &mut conn)
            .await?,
            FeedbackLabel::FalsePositive
        );

        Ok(())
    }

    struct MockAlert {
        reporter: &'static str,
        source: Option<&'static str>,
        token: String,
        url: Option<&'static str>,
    }

    impl SecretAlert for MockAlert {
        fn reporter(&self) -> &str {
            self.reporter
        }

        fn source(&self) -> Option<&str> {
            self.source
        }

        fn token(&self) -> &str {
            &self.token
        }

        fn url(&self) -> Option<&str> {
            self.url
        }
    }
}
