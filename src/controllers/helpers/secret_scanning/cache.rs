use std::{str::FromStr, sync::Arc};

use base64::{Engine, engine::general_purpose};
use chrono::{DateTime, Duration, Utc};
use p256::ecdsa::{VerifyingKey, signature::Verifier};

use crate::util::errors::{BoxedAppError, bad_request};

use super::{PublicKey, PublicKeyListSource};

/// A basic cache of public keys that can be used to verify signed payloads.
#[derive(Clone)]
pub struct PublicKeyCache<S>
where
    S: PublicKeyListSource,
    S::Key: PublicKey + 'static,
{
    source: S,
    ttl: Duration,
    keys: Vec<Arc<S::Key>>,
    timestamp: Option<DateTime<Utc>>,
}

impl<S> PublicKeyCache<S>
where
    S: PublicKeyListSource,
    S::Key: PublicKey + 'static,
{
    /// Instantiates a new public key cache.
    pub fn new(source: S, ttl: Duration) -> Self {
        Self {
            source,
            ttl,
            keys: Vec::new(),
            timestamp: None,
        }
    }

    /// Finds the given public key in the cache, updating it if necessary.
    pub async fn find(&mut self, id: &str) -> Result<Option<Arc<S::Key>>, BoxedAppError> {
        self.ensure_cache_is_valid().await?;
        Ok(self.keys.iter().find(|key| key.id() == id).cloned())
    }

    /// Verifies the given payload and signature against the keys in the cache, updating the cache
    /// if necessary.
    pub async fn verify(
        &mut self,
        id: &str,
        signature: &[u8],
        payload: &[u8],
    ) -> Result<(), BoxedAppError> {
        let signature = general_purpose::STANDARD
            .decode(signature)
            .map_err(|e| bad_request(format!("failed to decode signature as base64: {e:?}")))?;

        let signature = p256::ecdsa::Signature::from_der(&signature)
            .map_err(|e| bad_request(format!("failed to parse signature from ASN.1 DER: {e:?}")))?;

        let Some(key) = self.find(id).await? else {
            return Err(bad_request(format!("unknown key id {id}")));
        };

        if !key.is_current() {
            let error = bad_request(format!("key id {id} is not a current key"));
            return Err(error);
        }

        let public_key = p256::PublicKey::from_str(key.key())
            .map_err(|_| bad_request("cannot parse public key"))?;

        VerifyingKey::from(public_key)
            .verify(payload, &signature)
            .map_err(|e| bad_request(format!("invalid signature: {e:?}")))?;

        debug!(key_id = id, "secret alert request validated");

        Ok(())
    }

    async fn ensure_cache_is_valid(&mut self) -> Result<(), BoxedAppError> {
        if !self.is_cache_valid() {
            self.refresh_cache().await?;
        }

        Ok(())
    }

    fn is_cache_valid(&self) -> bool {
        self.timestamp
            .is_some_and(|timestamp| Utc::now() < (timestamp + self.ttl))
    }

    async fn refresh_cache(&mut self) -> Result<(), BoxedAppError> {
        self.keys = self.source.get().await?.into_iter().map(Arc::new).collect();
        self.timestamp = Some(Utc::now());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crates_io_github::GitHubPublicKey;

    use crate::controllers::helpers::secret_scanning::GenericPublicKey;

    use super::*;

    #[tokio::test]
    async fn test_is_cache_valid() -> Result<(), BoxedAppError> {
        fn new_cache(ttl: Duration) -> PublicKeyCache<MockSource> {
            PublicKeyCache::new(MockSource::default(), ttl)
        }

        // Unfilled cache.
        assert!(!new_cache(Duration::hours(24)).is_cache_valid());

        // Cache that was recently filled.
        let mut cache = new_cache(Duration::hours(24));
        cache.timestamp = Some(Utc::now());
        assert!(cache.is_cache_valid());

        // Cache that was filled a long time ago.
        let mut cache = new_cache(Duration::hours(24));
        cache.timestamp = Some(Utc::now() - cache.ttl * 2);
        assert!(!cache.is_cache_valid());

        // Cache that was filled in the mysterious future?
        let mut cache = new_cache(Duration::hours(24));
        cache.timestamp = Some(Utc::now() + cache.ttl * 2);
        assert!(cache.is_cache_valid());

        Ok(())
    }

    #[tokio::test]
    async fn test_find() -> Result<(), BoxedAppError> {
        static PUBLIC_KEY: &str = "keykeykey";

        let source = MockSource::new(&[("id-active", PUBLIC_KEY, true)]);
        let mut cache = PublicKeyCache::new(source, Duration::hours(24));

        assert_none!(cache.find("id-not-found").await?);

        let key = assert_some!(cache.find("id-active").await?);
        assert_eq!(key.id(), "id-active");
        assert_eq!(key.key(), PUBLIC_KEY);
        assert!(key.is_current());

        Ok(())
    }

    #[tokio::test]
    async fn test_verify() -> Result<(), BoxedAppError> {
        static PAYLOAD: &[u8] = br#"{"payload": {"token": "foobar"}}"#;

        // How did I generate this stuff?
        //
        // Private key: openssl ecparam -name prime256v1 -genkey -out ec_private.key
        // Public key:  openssl ec -in ec_private.key -pubout -out ec_public.key
        // Signature:   echo -n '{"payload": {"token": "foobar"}}' | openssl dgst -sha256 -sign ec_private.key | base64 -w 0
        //
        // Note that the public key must include the newline that separates the two lines of base64
        // output.
        static PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----\nMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEmf+tV9dbD3UKGtwcwt/Hf8vtW7yz\nXjcibIg1lZcyVHj3/mZV8eju0hQZGzgttCi/74Jkw1+CK/bsmcn3Ts8jtA==\n-----END PUBLIC KEY-----";
        static SIGNATURE: &[u8] = b"MEQCIAOvKzpbYFkSaXMvsTNNTem5isxxJHGMjqAyZC7yRi7pAiAiCrBjelYSXU/HhW6V4lLC9BeHsfasV0CXWgLsE2ch5Q==";

        // This was generated similarly to the above, and should never need to change.
        static OTHER_PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----\nMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE6NgBg4e0x2I2xmnCMETeAtRq3nfWgcK61Qr8ID8Vg1MS3eGgz33jpJysUnykBPuvzUITnur9JmsG3fqN23ZBUA==\n-----END PUBLIC KEY-----";

        let source = MockSource::new(&[
            ("inactive", PUBLIC_KEY, false),
            ("active", PUBLIC_KEY, true),
            ("different", OTHER_PUBLIC_KEY, true),
        ]);
        let mut cache = PublicKeyCache::new(source, Duration::hours(24));

        assert_ok!(cache.verify("active", SIGNATURE, PAYLOAD).await);
        assert_err!(cache.verify("inactive", SIGNATURE, PAYLOAD).await);
        assert_err!(cache.verify("different", SIGNATURE, PAYLOAD).await);
        assert_err!(cache.verify("not found", SIGNATURE, PAYLOAD).await);

        Ok(())
    }

    #[derive(Default)]
    struct MockSource {
        keys: Vec<GenericPublicKey>,
    }

    impl MockSource {
        fn new<'a>(keys: &[(&'a str, &'a str, bool)]) -> Self {
            Self {
                keys: keys
                    .iter()
                    .map(|(id, key, is_current)| {
                        GitHubPublicKey {
                            key_identifier: id.to_string(),
                            key: key.to_string(),
                            is_current: *is_current,
                        }
                        .into()
                    })
                    .collect(),
            }
        }
    }

    impl PublicKeyListSource for MockSource {
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
            Box::pin(std::future::ready(Ok(self.keys.clone())))
        }
    }
}
