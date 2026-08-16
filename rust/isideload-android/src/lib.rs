//! Android auth-layer wrapper around `isideload`'s Apple ID login flow.
//!
//! Scope on purpose: this crate only *calls* login + exports a session
//! blob. `idevice` (usbmuxd-feature only, no libusb) is linked in because
//! isideload's error type requires it at compile time, but nothing in
//! this crate's exported API ever opens a device connection. Real
//! device/USB usage -- and the root requirement that comes with it --
//! starts in a later phase (device layer, sideload/pairing layer).

use std::sync::{Arc, RwLock, Once};

use isideload::anisette::{AnisetteProvider, RemoteAnisetteProvider};
use isideload::auth::apple_account::{AppleAccount, TwoFactorCallbackResponse};

uniffi::setup_scaffolding!("isideload_android");

static INIT: Once = Once::new();

/// Must run exactly once before any login attempt -- without it,
/// isideload's network errors come back with no useful detail
/// (per the crate's own docs).
fn ensure_init() {
    INIT.call_once(|| {
        isideload::init();
    });
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum LoginError {
    #[error("login failed: {message}")]
    Failed { message: String },
    #[error("session could not be serialized: {message}")]
    Serialization { message: String },
}

#[derive(Debug, uniffi::Enum)]
pub enum LoginResult {
    /// Login complete. `session` is an opaque blob -- hand it to Kotlin
    /// to store via EncryptedSharedPreferences / Android Keystore.
    /// Do not attempt to parse it on the Kotlin side.
    Success { session: Vec<u8> },
}

/// What the login callback asks Kotlin to show the user, and what
/// Kotlin can respond with.
#[derive(Debug, uniffi::Enum)]
pub enum TwoFactorResponse {
    SubmitCode { code: String },
    ResendCode,
    Abort,
}

/// Implemented on the Kotlin side. `login()` below calls this mid-flow
/// if Apple requires a 2FA code -- Kotlin shows the entry screen and
/// suspends until the user responds.
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

    /// One call does the whole login, including any 2FA challenge.
    pub async fn login(
        &self,
        apple_id: String,
        password: String,
        handler: Arc<dyn TwoFactorHandler>,
    ) -> Result<LoginResult, LoginError> {
        let provider_instance = RemoteAnisetteProvider::new("https://anisette.v3.rs/".to_string());
        let provider = Arc::new(RwLock::new(provider_instance));
        let anisette_generator = AnisetteProvider::new(provider);

        let mut account = AppleAccount::new(&apple_id, anisette_generator, false, None)
            .await
            .map_err(|e| LoginError::Failed {
                message: e.to_string(),
            })?;

        account
            .login(&password, move |_params| {
                let handler = handler.clone();
                async move {
                    Ok(match handler.on_two_factor_required().await {
                        TwoFactorResponse::SubmitCode { code } => {
                            TwoFactorCallbackResponse::SubmitCode(code)
                        }
                        TwoFactorResponse::ResendCode => TwoFactorCallbackResponse::ResendCode,
                        TwoFactorResponse::Abort => TwoFactorCallbackResponse::Abort,
                    })
                }
            })
            .await
            .map_err(|e| LoginError::Failed {
                message: e.to_string(),
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
        message: e.to_string(),
    })
}
