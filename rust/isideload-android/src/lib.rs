//! Android auth-layer wrapper around `isideload`'s Apple ID login flow.

use std::sync::{Arc, Once};
use tokio::sync::RwLock;

use isideload::anisette::remote_v3::RemoteV3AnisetteProvider;
use isideload::anisette::{AnisetteDataGenerator, AnisetteProvider};
use isideload::auth::apple_account::{AppleAccount, TwoFactorCallbackResponse};
use isideload::util::storage::InMemoryStorage;

uniffi::setup_scaffolding!("isideload_android");

static INIT: Once = Once::new();

/// Uses SideStore's public anisette server to avoid TLS certificate issues.
const DEFAULT_ANISETTE_SERVER: &str = "https://ani.sidestore.io";

fn ensure_init() {
    INIT.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        isideload::init();
    });
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum LoginError {
    #[error("login failed: {reason}")]
    Failed { reason: String },
    #[error("session could not be serialized: {reason}")]
    Serialization { reason: String },
}

#[derive(Debug, uniffi::Enum)]
pub enum LoginResult {
    Success { session: Vec<u8> },
}

#[derive(Debug, uniffi::Enum)]
pub enum TwoFactorResponse {
    SubmitCode { code: String },
    ResendCode,
    Abort,
}

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait TwoFactorHandler: Send + Sync {
    async fn on_two_factor_required(&self) -> TwoFactorResponse;
}

#[derive(uniffi::Object)]
pub struct AuthSession;

#[uniffi::export(async_runtime = "tokio")]
impl AuthSession {
    #[uniffi::constructor]
    pub fn new() -> Self {
        ensure_init();
        Self
    }

    pub async fn login(
        &self,
        apple_id: String,
        password: String,
        _config_dir: String,
        _handler: Arc<dyn TwoFactorHandler>,
    ) -> Result<LoginResult, LoginError> {
        let provider = RemoteV3AnisetteProvider::new(
            DEFAULT_ANISETTE_SERVER,
            Box::new(InMemoryStorage::new()),
            "isideload-android-placeholder".to_string(),
        )
        .map_err(|e| LoginError::Failed {
            reason: e.to_string(),
        })?;

        let anisette_generator =
            AnisetteDataGenerator::new(Arc::new(RwLock::new(provider)) as Arc<RwLock<dyn AnisetteProvider + Send + Sync>>);

        let mut account = AppleAccount::new(&apple_id, anisette_generator)
            .await
            .map_err(|e| LoginError::Failed {
                reason: e.to_string(),
            })?;

        account
            .login(&password, Box::new(|_params| {
                TwoFactorCallbackResponse::Abort
            }))
            .await
            .map_err(|e| LoginError::Failed {
                reason: e.to_string(),
            })?;

        let session = serialize_session(&account)?;
        Ok(LoginResult::Success { session })
    }
}

#[derive(serde::Serialize)]
struct StoredSession {
    email: String,
    spd: Option<Vec<u8>>,
}

fn serialize_session(account: &AppleAccount) -> Result<Vec<u8>, LoginError> {
    let stored = StoredSession {
        email: account.email.clone(),
        spd: None,
    };
    serde_json::to_vec(&stored).map_err(|e| LoginError::Serialization {
        reason: e.to_string(),
    })
}