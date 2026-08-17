package com.classityreal.iloadandroid.auth

import uniffi.isideload_android.AuthSession
import uniffi.isideload_android.LoginResult as FfiLoginResult
import uniffi.isideload_android.LoginException as FfiLoginError
import uniffi.isideload_android.TwoFactorHandler
import uniffi.isideload_android.TwoFactorResponse
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

sealed class LoginOutcome {
    data class Success(val sessionBlob: ByteArray) : LoginOutcome()
    data class Failure(val message: String) : LoginOutcome()
}

class AuthBridge(private val configDir: String = "") : TwoFactorHandler {

    private val native = AuthSession()
    private var twoFactorCode: CompletableDeferred<String>? = null

    private val _twoFactorRequested = MutableStateFlow(false)
    val twoFactorRequested: StateFlow<Boolean> = _twoFactorRequested.asStateFlow()

    suspend fun login(appleId: String, password: String): LoginOutcome {
        return try {
            when (val result = native.login(appleId, password, configDir, this)) {
                is FfiLoginResult.Success -> LoginOutcome.Success(result.session)
            }
        } catch (e: FfiLoginError) {
            LoginOutcome.Failure(e.message ?: "login failed")
        } finally {
            _twoFactorRequested.value = false
        }
    }

    override suspend fun onTwoFactorRequired(): TwoFactorResponse {
        val deferred = CompletableDeferred<String>()
        twoFactorCode = deferred
        _twoFactorRequested.value = true
        val code = deferred.await()
        return TwoFactorResponse.SubmitCode(code)
    }

    fun submitTwoFactorCode(code: String) {
        twoFactorCode?.complete(code)
    }
}