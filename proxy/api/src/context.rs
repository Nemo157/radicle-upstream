//! Datastructure and machinery to safely share the common dependencies across components.

use std::sync::Arc;

use data_encoding::HEXLOWER;
use rand::Rng as _;
use tokio::sync::RwLock;

use coco::PeerControl;

use crate::service;

#[cfg(test)]
use coco::{signer, RunConfig};

/// Container to pass down dependencies into HTTP filter chains.
#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Context {
    /// Coco peer API is sealed and unavailable
    Sealed(Sealed),
    /// Coco peer API is unsealed and available
    Unsealed(Unsealed),
}

impl Context {
    /// Returns `true` if the stack is set up in test mode.
    pub const fn test(&self) -> bool {
        match self {
            Self::Sealed(sealed) => sealed.test,
            Self::Unsealed(unsealed) => unsealed.test,
        }
    }

    /// Returns the [`kv::Store`] for persistent storage.
    pub const fn store(&self) -> &kv::Store {
        match self {
            Self::Sealed(sealed) => &sealed.store,
            Self::Unsealed(unsealed) => &unsealed.store,
        }
    }

    /// Returns a mutable reference to the authentication cookie value.
    pub fn auth_token(&self) -> Arc<RwLock<Option<String>>> {
        match self {
            Self::Sealed(sealed) => sealed.auth_token.clone(),
            Self::Unsealed(unsealed) => unsealed.auth_token.clone(),
        }
    }

    /// Returns a handle to control the service configuration
    pub fn service_handle(&mut self) -> &mut service::Handle {
        match self {
            Self::Sealed(sealed) => &mut sealed.service_handle,
            Self::Unsealed(unsealed) => &mut unsealed.service_handle,
        }
    }

    /// Unseal the key store and restart the coco service with the obtained key. Returns the auth
    /// token required to access the keystore.
    ///
    /// # Errors
    ///
    /// * Errors if the passphrase is wrong.
    /// * Errors if backend fails to retrieve the data.
    /// * Errors if there is no key in the storage yet.
    pub async fn unseal_keystore(
        &mut self,
        passphrase: coco::keystore::SecUtf8,
    ) -> Result<String, crate::error::Error> {
        let keystore = self.keystore();
        let key = tokio::task::spawn_blocking(move || keystore.get(passphrase))
            .await
            .expect("Task to unseal key was aborted")?;
        self.service_handle().set_secret_key(key);
        let auth_token = self.reset_auth_token().await;
        Ok(auth_token)
    }

    /// Create a key and store it encrypted with the given passphrase. Then restart the coco
    /// service to use the new key. Returns the auth token required to access the keystore.
    ///
    /// # Errors
    ///
    /// Errors when the storage backend fails to persist the key or a key already exists.
    pub async fn create_key(
        &mut self,
        passphrase: coco::keystore::SecUtf8,
    ) -> Result<String, crate::error::Error> {
        let keystore = self.keystore();
        let key = tokio::task::spawn_blocking(move || keystore.create_key(passphrase))
            .await
            .expect("Task to create key was aborted")?;
        self.service_handle().set_secret_key(key);
        let auth_token = self.reset_auth_token().await;
        Ok(auth_token)
    }

    fn keystore(&self) -> Arc<dyn coco::keystore::Keystore + Sync + Send> {
        match self {
            Self::Sealed(sealed) => sealed.keystore.clone(),
            Self::Unsealed(unsealed) => unsealed.keystore.clone(),
        }
    }

    /// Generate a new authentication token and store it.
    async fn reset_auth_token(&self) -> String {
        let new_token_data = rand::thread_rng().gen::<[u8; 32]>();
        let new_token = HEXLOWER.encode(&new_token_data);
        let auth_token_lock = self.auth_token();
        let mut auth_token = auth_token_lock.write().await;
        *auth_token = Some(new_token.clone());
        new_token
    }

    /// Returns `true` if `token` matches the stored authentication token.
    pub async fn check_auth_token(&self, token: Option<String>) -> bool {
        token == *self.auth_token().read().await
    }
}

impl From<Unsealed> for Context {
    fn from(unsealed: Unsealed) -> Self {
        Self::Unsealed(unsealed)
    }
}

impl From<Sealed> for Context {
    fn from(sealed: Sealed) -> Self {
        Self::Sealed(sealed)
    }
}

/// Context for HTTP requests with access to coco peer APIs.
#[derive(Clone)]
pub struct Unsealed {
    /// Handle to inspect state and perform actions on the currently running local [`coco::Peer`].
    pub peer_control: PeerControl,
    /// [`coco::State`] to operate on the local monorepo.
    pub state: coco::State,
    /// [`kv::Store`] used for session state and cache.
    pub store: kv::Store,
    /// Flag to control if the stack is set up in test mode.
    pub test: bool,
    /// Handle to control the service configuration.
    pub service_handle: service::Handle,
    /// Cookie set on unsealing the key store.
    pub auth_token: Arc<RwLock<Option<String>>>,
    /// Reference to the key store.
    pub keystore: Arc<dyn coco::keystore::Keystore + Send + Sync>,
}

/// Context for HTTP request if the coco peer APIs have not been initialized yet.
#[derive(Clone)]
pub struct Sealed {
    /// [`kv::Store`] used for session state and cache.
    pub store: kv::Store,
    /// Flag to control if the stack is set up in test mode.
    pub test: bool,
    /// Handle to control the service configuration.
    pub service_handle: service::Handle,
    /// Cookie set on unsealing the key store.
    pub auth_token: Arc<RwLock<Option<String>>>,
    /// Reference to the key store.
    pub keystore: Arc<dyn coco::keystore::Keystore + Send + Sync>,
}

impl Unsealed {
    /// Initialises a new [`Unsealed`] context with the store and coco state in the given temporary
    /// directory.
    ///
    /// # Errors
    ///
    /// * coco key creation fails
    /// * creation of the [`kv::Store`] fails
    #[cfg(test)]
    pub async fn tmp(tmp_dir: &tempfile::TempDir) -> Result<Self, crate::error::Error> {
        let store = kv::Store::new(kv::Config::new(tmp_dir.path().join("store")))?;

        let key = coco::keys::SecretKey::new();
        let signer = signer::BoxedSigner::from(signer::SomeSigner { signer: key });

        let (peer_control, state) = {
            let config = coco::config::default(key, tmp_dir.path())?;
            let (peer, state) =
                coco::into_peer_state(config, signer.clone(), store.clone(), RunConfig::default())
                    .await?;

            let peer_control = peer.control();
            tokio::spawn(peer.into_running());

            (peer_control, state)
        };

        Ok(Self {
            peer_control,
            state,
            store,
            test: false,
            service_handle: service::Handle::dummy(),
            auth_token: Arc::new(RwLock::new(None)),
            keystore: Arc::new(coco::keystore::memory()),
        })
    }
}
