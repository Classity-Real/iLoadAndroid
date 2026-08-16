package com.classityreal.iloadandroid.auth

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.launch

enum class LoginStep { CREDENTIALS, TWO_FACTOR, DONE }

class LoginViewModel(
    private val authBridge: AuthBridge = AuthBridge()
) : ViewModel() {

    var step by mutableStateOf(LoginStep.CREDENTIALS)
        private set
    var isLoading by mutableStateOf(false)
        private set
    var errorMessage by mutableStateOf<String?>(null)
        private set

    /** Set once login succeeds. Caller (MainActivity/nav layer) is responsible
     *  for persisting this via Android Keystore-backed storage — this
     *  ViewModel does not touch disk. */
    var sessionBlob by mutableStateOf<ByteArray?>(null)
        private set

    fun submitCredentials(appleId: String, password: String) {
        if (appleId.isBlank() || password.isBlank()) {
            errorMessage = "Enter both an Apple ID and password"
            return
        }
        errorMessage = null
        isLoading = true
        viewModelScope.launch {
            when (val outcome = authBridge.login(appleId, password)) {
                is LoginOutcome.Success -> {
                    sessionBlob = outcome.sessionBlob
                    step = LoginStep.DONE
                }
                is LoginOutcome.TwoFactorRequired -> {
                    step = LoginStep.TWO_FACTOR
                }
                is LoginOutcome.Failure -> {
                    errorMessage = outcome.message
                }
            }
            isLoading = false
        }
    }

    fun submitTwoFactorCode(code: String) {
        if (code.length != 6) {
            errorMessage = "Enter the 6-digit code"
            return
        }
        errorMessage = null
        isLoading = true
        viewModelScope.launch {
            when (val outcome = authBridge.submit2fa(code)) {
                is LoginOutcome.Success -> {
                    sessionBlob = outcome.sessionBlob
                    step = LoginStep.DONE
                }
                is LoginOutcome.TwoFactorRequired -> {
                    errorMessage = "Code rejected, try again"
                }
                is LoginOutcome.Failure -> {
                    errorMessage = outcome.message
                }
            }
            isLoading = false
        }
    }
}
