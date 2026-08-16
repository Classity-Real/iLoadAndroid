//! Android auth-layer wrapper around `isideload`'s Apple ID login flow.
//! ...
use std::sync::{Mutex, Once};

// Import the types from their module paths in the isideload crate.
// isideload does not re-export these at crate root, so use the correct modules.
// AppleAccount is defined under isideload::auth::apple_account and
// AnisetteConfiguration under isideload::anisette::remote_v3.
use isideload::auth::apple_account::AppleAccount;
use isideload::anisette::remote_v3::AnisetteConfiguration;

uniffi::setup_scaffolding!("isideload_android");

static INIT: Once = Once::new();

/// Must run exactly once before any login attempt — without it,
/// isideload's network errors come back with no useful detail
/// (per the crate's own docs).
fn ensure_init() {
    INIT.call_once(|| {
        isideload::init();
    });
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum LoginError {
    #[error("invalid Apple ID or password")]
    InvalidCredentials,
    #[error("two-factor code was incorrect")]
    Invalid2faCode,
    #[error("no login attempt in progress")]
    NoPendingLogin,
    #[error("anisette/network error: {message}")]
    Network { message: String },
    #[error("session could not be serialized: {message}")]
    Serialization { message: String },
}

#[derive(Debug, uniffi::Enum)]
pub enum LoginResult {
    /// Login complete. `session` is an opaque blob — hand it to Kotlin
    /// to store via EncryptedSharedPreferences / Android Keystore.
    /// Do not attempt to parse it on the Kotlin side.
    Success { session: Vec<u8> },
    /// Apple ID + password were accepted, but a 2FA code sent to a
    /// trusted device is required. Call `submit_2fa` next.
    TwoFactorRequired,
}

/// Holds in-progress login state between `login()` and `submit_2fa()`,
/// since Apple ID auth is a two-step challenge/response flow.
#[derive(uniffi::Object)]
pub struct AuthSession {
    // VERIFY: confirm AppleAccount is actually the right in-progress
    // handle to hold here, and that it's what needs the 2FA code
    // submitted back to it (vs. a separate intermediate type).
    pending: Mutex<Option<AppleAccount>>,
}

#[uniffi::export(async_runtime = "tokio")]
impl AuthSession {
    #[uniffi::constructor]
    pub fn new() -> Self {
        ensure_init();
        Self {
            pending: Mutex::new(None),
        }
    }

    /// Start (or restart) a login attempt with an Apple ID + password.
    pub async fn login(
        &self,
        apple_id: String,
        password: String,
    ) -> Result<LoginResult, LoginError> {
        // VERIFY: exact constructor/config for AnisetteConfiguration.
        let anisette_config = AnisetteConfiguration::default();

        let login_attempt = AppleAccount::login(apple_id, password, anisette_config)
            .await
            .map_err(|e| LoginError::Network {
                message: e.to_string(),
            })?;

        if login_attempt.needs_2fa() {
            *self.pending.lock().unwrap() = Some(login_attempt);
            Ok(LoginResult::TwoFactorRequired)
        } else {
            let bytes = serialize_session(&login_attempt)?;
            Ok(LoginResult::Success { session: bytes })
        }
    }

    /// Submit the 6-digit code shown on a trusted device to complete
    /// a login that returned `TwoFactorRequired`.
    pub async fn submit_2fa(&self, code: String) -> Result<LoginResult, LoginError> {
        let mut guard = self.pending.lock().unwrap();
        let account = guard.as_mut().ok_or(LoginError::NoPendingLogin)?;

        account
            .submit_2fa_code(code)
            .await
            .map_err(|_| LoginError::Invalid2faCode)?;

        let bytes = serialize_session(account)?;
        *guard = None;
        Ok(LoginResult::Success { session: bytes })
    }
}

/// Serialize a completed AppleAccount session to an opaque byte blob.
fn serialize_session(account: &AppleAccount) -> Result<Vec<u8>, LoginError> {
    serde_json::to_vec(account).map_err(|e| LoginError::Serialization {
        message: e.to_string(),
    })
}
