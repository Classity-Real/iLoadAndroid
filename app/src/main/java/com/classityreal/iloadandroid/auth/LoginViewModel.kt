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
    var sessionBlob by mutableStateOf<ByteArray?>(null)
        private set

    init {
        viewModelScope.launch {
            authBridge.twoFactorRequested.collect { requested ->
                if (requested) {
                    step = LoginStep.TWO_FACTOR
                    isLoading = false
                }
            }
        }
    }

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
                    isLoading = false
                }
                is LoginOutcome.Failure -> {
                    errorMessage = outcome.message
                    step = LoginStep.CREDENTIALS
                    isLoading = false
                }
            }
        }
    }

    fun submitTwoFactorCode(code: String) {
        if (code.length != 6) {
            errorMessage = "Enter the 6-digit code"
            return
        }
        errorMessage = null
        isLoading = true
        authBridge.submitTwoFactorCode(code)
        // The still-running submitCredentials() coroutine's login() call
        // unblocks here and delivers the real result via its own when-branch.
    }
}