use async_trait::async_trait;

use crate::util::errors::BoxedAppError;

use super::PublicKey;

/// A source of public keys that should be cached.
#[async_trait]
pub trait PublicKeyListSource: Send + Sync {
    type Key: PublicKey;

    // This is a bit awkward: we have to use async-trait to make this dyn-compatible, but the
    // generated type signature is horrifying. Practically, the only sensible thing to do is to
    // implement this by immediately delegating to an async fn that actually makes sense. Hopefully
    // we get dyn-compatible async traits in the language soon and this can go away.
    async fn get(&self) -> Result<Vec<Self::Key>, BoxedAppError>;
}
