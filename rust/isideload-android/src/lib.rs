//! Android auth-layer wrapper around `isideload`'s Apple ID login flow.
//!
//! Scope on purpose: this crate only *calls* login + exports a session
//! blob. `idevice` (usbmuxd-feature only, no libusb) is linked in because
//! isideload's error type requires it at compile time, but nothing in
//! this crate's exported API ever opens a device connection. Real
//! device/USB usage -- and the root requirement that comes with it --
//! starts in a later phase (device layer, sideload/pairing layer).
//!
//! API confirmed against docs.rs/isideload/0.3.17 (auth::apple_account
//! module) -- not guessed from README examples, which are stale for
//! this version. One piece is still unconfirmed and marked VERIFY:
//! the real constructor for `AnisetteDataGenerator`. Everything else
//! below (AppleAccount::new/login, TwoFactorCallbackResponse, the lack
//! of a `submit_2fa_code` method, and the lack of `Serialize` on
//! AppleAccount) is confirmed from the actual struct/method docs.

use std::sync::{Arc, Once};

use isideload::anisette::AnisetteDataGenerator;
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
/// Kotlin can respond with. Intentionally minimal for now: the real
/// `TwoFactorCallbackParams` type's exact fields (trusted device list,
/// phone numbers, etc.) haven't been confirmed against the source yet
/// -- this covers the common "enter the code you were sent" case only.
/// Widen this once TwoFactorCallbackParams's fields are confirmed.
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

    /// One call does the whole login, including any 2FA challenge --
    /// isideload's `AppleAccount::login` takes the 2FA callback
    /// directly rather than returning a "needs 2FA" state to poll,
    /// so there is no separate submit-code method to call afterward.
    pub async fn login(
        &self,
        apple_id: String,
        password: String,
        handler: Arc<dyn TwoFactorHandler>,
    ) -> Result<LoginResult, LoginError> {
        // VERIFY: real constructor for AnisetteDataGenerator. Likely
        // takes an anisette server URL (iloader/isideload default to
        // a hosted server -- errors in isideload's issue tracker
        // reference `ani.sidestore.io`) but the exact builder/method
        // name needs confirming against isideload::anisette source
        // before this compiles.
        let anisette_generator = AnisetteDataGenerator::default();

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

/// AppleAccount does NOT implement Serialize (confirmed -- it holds an
/// Arc<GrandSlam> network client, which can't be serialized). Persist
/// only what's needed to identify the session: email + the session
/// provisioning dictionary (`spd`). Reconstructing a full AppleAccount
/// from this blob for a later sideload/cert-management phase is a
/// separate, not-yet-written piece of work.
#[derive(serde::Serialize)]
struct StoredSession {
    email: String,
    spd: Option<Vec<u8>>, // VERIFY: plist::Dictionary -> bytes encoding TBD
}

fn serialize_session(account: &AppleAccount) -> Result<Vec<u8>, LoginError> {
    let stored = StoredSession {
        email: account.email.clone(),
        spd: None, // VERIFY: encode account.spd (plist::Dictionary) once needed
    };
    serde_json::to_vec(&stored).map_err(|e| LoginError::Serialization {
        message: e.to_string(),
    })
}
