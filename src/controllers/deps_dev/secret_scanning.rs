use std::sync::OnceLock;

use axum::{Json, body::Bytes};
use chrono::Duration;
use http::HeaderMap;
use tokio::sync::Mutex;

use crate::{
    app::AppState,
    controllers::helpers::secret_scanning::{
        FeedbackLabel, GenericPublicKey, PublicKeyCache, PublicKeyListSource, SecretAlert,
    },
    util::errors::{AppResult, BoxedAppError, bad_request},
};

// The deps.dev endpoint to get public keys from.
const DEPS_DEV_PUBLIC_KEY_URL: &str =
    "https://storage.googleapis.com/depsdev-gcp-public-keys/secret_scanning";

// How long to wait before refreshing the deps.dev public key cache.
const PUBLIC_KEY_CACHE_LIFETIME: Duration = Duration::hours(24);

// Cache of public keys that have been fetched from the deps.dev API.
fn public_key_cache(state: &AppState) -> &'static Mutex<PublicKeyCache<Source>> {
    static LOCK: OnceLock<Mutex<PublicKeyCache<Source>>> = OnceLock::new();

    LOCK.get_or_init(|| Mutex::new(PublicKeyCache::new(Source, PUBLIC_KEY_CACHE_LIFETIME)))
}

struct Source;

impl Source {
    async fn get(&self) -> Result<Vec<GenericPublicKey>, BoxedAppError> {
        Ok(reqwest::Client::builder()
            .user_agent("crates.io (https://crates.io)")
            .build()?
            .get(DEPS_DEV_PUBLIC_KEY_URL)
            .send()
            .await?
            .json()
            .await?)
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

/// Verifies that the deps.dev signature in the request headers is valid.
async fn verify_deps_dev_signature(
    cache: &mut PublicKeyCache<Source>,
    headers: &HeaderMap,
    json: &[u8],
) -> Result<(), BoxedAppError> {
    const KEY_ID_HEADER: &str = "DepsDev-Public-Key-Identifier";
    const SIGNATURE_HEADER: &str = "DepsDev-Public-Key-Signature";

    let req_key_id = headers
        .get(KEY_ID_HEADER)
        .ok_or_else(|| bad_request(format!("missing HTTP header: {KEY_ID_HEADER}")))?
        .to_str()
        .map_err(|e| bad_request(format!("failed to decode HTTP header: {e:?}")))?;

    let sig = headers
        .get(SIGNATURE_HEADER)
        .ok_or_else(|| bad_request(format!("missing HTTP header: {SIGNATURE_HEADER}")))?;

    cache.verify(req_key_id, sig.as_bytes(), json).await
}

#[derive(Deserialize, Serialize)]
struct DepsDevSecretAlert {
    token: String,
    url: String,
}

impl SecretAlert for DepsDevSecretAlert {
    fn reporter(&self) -> &str {
        "deps.dev"
    }

    fn source(&self) -> Option<&str> {
        None
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
pub struct DepsDevSecretAlertFeedback {
    pub token_raw: String,
    pub label: FeedbackLabel,
}

/// Handles the `POST /api/deps.dev/secret-scanning/verify` route.
pub async fn verify(
    state: AppState,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<Json<Vec<DepsDevSecretAlertFeedback>>> {
    let mut cache = public_key_cache(&state).lock().await;

    verify_deps_dev_signature(&mut cache, &headers, &body)
        .await
        .map_err(|e| bad_request(format!("failed to verify request signature: {e:?}")))?;

    let alerts: Vec<DepsDevSecretAlert> = serde_json::from_slice(&body)
        .map_err(|e| bad_request(format!("invalid secret alert request: {e:?}")))?;

    let mut conn = state.db_write().await?;

    let mut feedback = Vec::with_capacity(alerts.len());
    for alert in alerts {
        let label = alert.revoke_token(&state, &mut conn).await?;
        feedback.push(DepsDevSecretAlertFeedback {
            token_raw: alert.token,
            label,
        });
    }

    Ok(Json(feedback))
}
