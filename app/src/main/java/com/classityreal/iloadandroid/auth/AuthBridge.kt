package com.classityreal.iloadandroid.auth

import uniffi.isideload_android.AuthSession
import uniffi.isideload_android.LoginResult as FfiLoginResult
import uniffi.isideload_android.LoginError as FfiLoginError
import uniffi.isideload_android.TwoFactorHandler
import uniffi.isideload_android.TwoFactorResponse
import kotlinx.coroutines.CompletableDeferred

sealed class LoginOutcome {
    data class Success(val sessionBlob: ByteArray) : LoginOutcome()
    data object TwoFactorRequired : LoginOutcome()
    data class Failure(val message: String) : LoginOutcome()
}

/**
 * App-facing auth API. Holds one native AuthSession per login attempt;
 * the underlying object keeps the in-progress 2FA state, so this class
 * must not be recreated between `login()` and `submit2fa()` calls.
 */
class AuthBridge(private val configDir: String = "") : TwoFactorHandler {

    private val native = AuthSession()
    private val twoFactorCode = CompletableDeferred<String>()

    suspend fun login(appleId: String, password: String): LoginOutcome {
        return try {
            when (val result = native.login(appleId, password, configDir, this)) {
                is FfiLoginResult.Success -> LoginOutcome.Success(result.session)
            }
        } catch (e: FfiLoginError) {
            LoginOutcome.Failure(e.message ?: "login failed")
        }
    }

    override suspend fun onTwoFactorRequired(): TwoFactorResponse {
        // This is called by the Rust side when 2FA is needed.
        // We wait for the user to provide the code via submit2fa().
        val code = twoFactorCode.await()
        return TwoFactorResponse.SubmitCode(code)
    }

    suspend fun submit2fa(code: String): LoginOutcome {
        twoFactorCode.complete(code)
        // We return Success here as a placeholder; the actual result of the login
        // flow is returned by the original login() call.
        return LoginOutcome.Success(byteArrayOf())
    }
}
