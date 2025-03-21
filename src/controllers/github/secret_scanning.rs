use crate::app::AppState;
use crate::controllers::helpers::secret_scanning::{
    FeedbackLabel, GenericPublicKey, PublicKeyCache, PublicKeyListSource, SecretAlert,
};
use crate::util::errors::{AppResult, BoxedAppError, bad_request};
use axum::Json;
use axum::body::Bytes;
use chrono::Duration;
use http::HeaderMap;
use serde_json as json;
use std::sync::OnceLock;
use tokio::sync::Mutex;

// How long to wait before refreshing cache of GitHub's public keys.
const PUBLIC_KEY_CACHE_LIFETIME: Duration = Duration::hours(24);

// Cache of public keys that have been fetched from GitHub API
fn public_key_cache(state: &AppState) -> &'static Mutex<PublicKeyCache<Source>> {
    static LOCK: OnceLock<Mutex<PublicKeyCache<Source>>> = OnceLock::new();

    LOCK.get_or_init(|| {
        Mutex::new(PublicKeyCache::new(
            Source {
                state: state.clone(),
            },
            PUBLIC_KEY_CACHE_LIFETIME,
        ))
    })
}

struct Source {
    state: AppState,
}

impl Source {
    async fn get(&self) -> Result<Vec<GenericPublicKey>, BoxedAppError> {
        // Fetch from GitHub API
        let client_id = &self.state.config.gh_client_id;
        let client_secret = self.state.config.gh_client_secret.secret();
        let keys = self
            .state
            .github
            .public_keys(client_id, client_secret)
            .await?;

        Ok(keys.into_iter().map(GenericPublicKey::from).collect())
    }
}

impl PublicKeyListSource for Source {
    type Key = GenericPublicKey;

    #[must_use]
    #[allow(
        elided_named_lifetimes,
        clippy::type_complexity,
        clippy::type_repetition_in_bounds
    )]
    fn get<'life0, 'async_trait>(
        &'life0 self,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<Vec<Self::Key>, BoxedAppError>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(self.get())
    }
}

/// Verifies that the GitHub signature in request headers is valid
async fn verify_github_signature(
    cache: &mut PublicKeyCache<Source>,
    headers: &HeaderMap,
    json: &[u8],
) -> Result<(), BoxedAppError> {
    // Read and decode request headers
    let req_key_id = headers
        .get("GITHUB-PUBLIC-KEY-IDENTIFIER")
        .ok_or_else(|| bad_request("missing HTTP header: GITHUB-PUBLIC-KEY-IDENTIFIER"))?
        .to_str()
        .map_err(|e| bad_request(format!("failed to decode HTTP header: {e:?}")))?;

    let sig = headers
        .get("GITHUB-PUBLIC-KEY-SIGNATURE")
        .ok_or_else(|| bad_request("missing HTTP header: GITHUB-PUBLIC-KEY-SIGNATURE"))?;

    cache.verify(req_key_id, sig.as_bytes(), json).await
}

#[derive(Deserialize, Serialize)]
struct GitHubSecretAlert {
    token: String,
    r#type: String,
    url: String,
    source: String,
}

impl SecretAlert for GitHubSecretAlert {
    fn reporter(&self) -> &str {
        "GitHub"
    }

    fn source(&self) -> Option<&str> {
        if self.source.is_empty() {
            None
        } else {
            Some(&self.source)
        }
    }

    fn token(&self) -> &str {
        &self.token
    }

    fn url(&self) -> Option<&str> {
        if self.url.is_empty() {
            None
        } else {
            Some(&self.url)
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct GitHubSecretAlertFeedback {
    pub token_raw: String,
    pub token_type: String,
    pub label: FeedbackLabel,
}

/// Handles the `POST /api/github/secret-scanning/verify` route.
pub async fn verify(
    state: AppState,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<Json<Vec<GitHubSecretAlertFeedback>>> {
    let mut cache = public_key_cache(&state).lock().await;

    verify_github_signature(&mut cache, &headers, &body)
        .await
        .map_err(|e| bad_request(format!("failed to verify request signature: {e:?}")))?;

    let alerts: Vec<GitHubSecretAlert> = json::from_slice(&body)
        .map_err(|e| bad_request(format!("invalid secret alert request: {e:?}")))?;

    let mut conn = state.db_write().await?;

    let mut feedback = Vec::with_capacity(alerts.len());
    for alert in alerts {
        let label = alert.revoke_token(&state, &mut conn).await?;
        feedback.push(GitHubSecretAlertFeedback {
            token_raw: alert.token,
            token_type: alert.r#type,
            label,
        });
    }

    Ok(Json(feedback))
}
