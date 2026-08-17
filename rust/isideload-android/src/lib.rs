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
//! this version.
//!
//! `AnisetteDataGenerator::new()` takes an
//! `Arc<RwLock<dyn AnisetteProvider + Send + Sync>>` (confirmed from a
//! compiler error, not docs). The concrete provider is
//! `isideload::anisette::remote_v3::RemoteV3AnisetteProvider` --
//! confirmed directly from a compiler suggestion against the real
//! module (not inferred/guessed), after an earlier wrong guess at the
//! name (`RemoteAnisetteProviderV3`, transposed) failed to compile.
//! Constructor signature `new(url: &str, storage: Box<dyn
//! SideloadingStorage>, serial_number: String) -> Result<Self, Report>`
//! is confirmed directly against docs.rs/isideload/0.3.17
//! (isideload::anisette::remote_v3::RemoteV3AnisetteProvider), not
//! inferred from the upstream `omnisette` crate it's presumed to wrap.

use std::sync::{Arc, Once};
use tokio::sync::RwLock;

use isideload::anisette::remote_v3::RemoteV3AnisetteProvider;
use isideload::anisette::{AnisetteDataGenerator, AnisetteProvider};
use isideload::auth::apple_account::{AppleAccount, TwoFactorCallbackResponse};
use isideload::util::storage::InMemoryStorage;

/// Default anisette provisioning server. iloader/isideload's own default
/// (referenced in isideload's issue tracker when this server has an
/// outage) -- VERIFY this is still current before shipping.
const DEFAULT_ANISETTE_SERVER: &str = "https://ani.sidestore.io";

uniffi::setup_scaffolding!("isideload_android");

static INIT: Once = Once::new();

/// Must run exactly once before any login attempt -- without it,
/// isideload's network errors come back with no useful detail
/// (per the crate's own docs).
fn ensure_init() {
    INIT.call_once(|| {
        // Install the 'ring' cryptographic backend for rustls globally.
        // We use `let _ =` because it will return an error if called multiple times, but `Once` guarantees it only runs on the first pass anyway.
        let _ = rustls::crypto::ring::default_provider().install_default();

        // Initialize isideload. The HTTP client configuration is handled internally by isideload.
        // The previous attempt to configure the HTTP client manually caused errors because
        // the `isideload::util::http` module does not exist in the current version of isideload.
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
    ///
    /// `config_dir` must be a writable directory -- pass Android's
    /// app-specific files dir (`context.filesDir.path` on the Kotlin
    /// side) since Rust has no way to know that path itself. It's
    /// where the anisette provider caches provisioning state between
    /// calls; VERIFY it's actually used that way once this compiles.
    pub async fn login(
        &self,
        apple_id: String,
        password: String,
        _config_dir: String, // unused -- reserved for a future persistent SideloadingStorage impl
        handler: Arc<dyn TwoFactorHandler>,
    ) -> Result<LoginResult, LoginError> {
        // VERIFY: constructor argument names/order are inferred from
        // omnisette's RemoteAnisetteProviderV3 (which this type is
        // presumed to wrap under a different name) -- `serial` is a
        // device serial/identifier string. No real device exists yet
        // at this auth-only stage, so this is a placeholder per-install
        // identifier.
        //
        // Second arg is Box<dyn SideloadingStorage> (confirmed via
        // isideload::util::storage::SideloadingStorage) -- a small
        // trait (store/retrieve by string key, plus byte-oriented
        // helpers) for persisting anisette provisioning state and
        // certs. isideload ships InMemoryStorage as a ready-made
        // implementor, used below.
        // In-memory only for now: anisette provisioning state does not
        // persist across process restarts, so a fresh provisioning
        // round-trip happens on every cold login. Fine for the
        // auth-only scope of this phase -- swap for a persistent
        // (e.g. EncryptedSharedPreferences-backed) SideloadingStorage
        // impl once provisioning-state persistence actually matters.
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

        let mut account = AppleAccount::new(&apple_id, anisette_generator, false, None)
            .await
            .map_err(|e| LoginError::Failed {
                reason: e.to_string(),
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
                reason: e.to_string(),
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
        reason: e.to_string(),
    })
}
