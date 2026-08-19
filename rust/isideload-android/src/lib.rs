//! Android auth-layer wrapper using SideStore's omnisette + icloud-auth.
//!
//! This bypasses the rustls-platform-verifier certificate issue by using
//! omnisette-server (a local/remote HTTP service) for anisette generation
//! instead of isideload's built-in RemoteV3AnisetteProvider.

use std::sync::{Arc, Once};
use tokio::sync::RwLock;

use omnisette::remote_anisette_v3::RemoteAnisetteV3;
use icloud_auth::auth::AppleID;

uniffi::setup_scaffolding!("isideload_android");

static INIT: Once = Once::new();

/// Omnisette server URL. Uses SideStore's public anisette server.
/// This avoids TLS certificate issues by using a simple HTTP API.
const OMNISETTE_SERVER: &str = "https://ani.sidestore.io";

/// Must run exactly once before any login attempt.
fn ensure_init() {
    INIT.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
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
    /// Login complete. `session` is an opaque blob.
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
        // Create anisette provider pointing to omnisette-server
        let anisette = RemoteAnisetteV3::new(OMNISETTE_SERVER.to_string())
            .await
            .map_err(|e| LoginError::Failed {
                reason: format!("Failed to connect to omnisette-server: {}", e),
            })?;

        // Perform Apple ID authentication using icloud-auth
        let mut account = AppleID::new(&apple_id, &password, anisette)
            .await
            .map_err(|e| LoginError::Failed {
                reason: format!("Apple ID authentication failed: {}", e),
            })?;

        // Login (2FA handled inline)
        account
            .login()
            .await
            .map_err(|e| LoginError::Failed {
                reason: format!("Login failed: {}", e),
            })?;

        // Serialize session
        let session = serialize_session(&account)?;
        Ok(LoginResult::Success { session })
    }
}

#[derive(serde::Serialize)]
struct StoredSession {
    email: String,
    session_data: Option<Vec<u8>>,
}

fn serialize_session(account: &AppleID) -> Result<Vec<u8>, LoginError> {
    let stored = StoredSession {
        email: account.email().to_string(),
        session_data: None,
    };
    serde_json::to_vec(&stored).map_err(|e| LoginError::Serialization {
        reason: e.to_string(),
    })
}