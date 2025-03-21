use std::fmt::Debug;

use crates_io_github::GitHubPublicKey;

/// A public key published by an upstream provider that is using it to sign payloads.
pub trait PublicKey: Debug + Send + Sync {
    fn id(&self) -> &str;
    fn key(&self) -> &str;
    fn is_current(&self) -> bool;
}

/// A public key in the format published by GitHub, as documented at
/// https://docs.github.com/en/code-security/secret-scanning/secret-scanning-partnership-program/secret-scanning-partner-program#implement-signature-verification-in-your-secret-alert-service.
///
/// Note that this is not necessarily exclusive to GitHub; other services may decide to implement
/// the same format, as deps.dev has.
///
/// This is essentially just a newtype for [`crates_io_github::GitHubPublicKey`] that implements
/// [`PublicKey`].
#[derive(Debug, Deserialize, Clone, Eq, Hash, PartialEq)]
pub struct GenericPublicKey(GitHubPublicKey);

impl From<GitHubPublicKey> for GenericPublicKey {
    fn from(value: GitHubPublicKey) -> Self {
        Self(value)
    }
}

impl PublicKey for GenericPublicKey {
    fn id(&self) -> &str {
        &self.0.key_identifier
    }

    fn key(&self) -> &str {
        &self.0.key
    }

    fn is_current(&self) -> bool {
        self.0.is_current
    }
}
