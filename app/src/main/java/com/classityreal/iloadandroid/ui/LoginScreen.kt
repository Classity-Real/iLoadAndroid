package com.classityreal.iloadandroid.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.classityreal.iloadandroid.auth.LoginStep
import com.classityreal.iloadandroid.auth.LoginViewModel

@Composable
fun LoginScreen(
    onLoggedIn: (ByteArray) -> Unit,
    viewModel: LoginViewModel = viewModel()
) {
    Scaffold { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(24.dp),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Text(
                text = "Sign in with Apple ID",
                style = MaterialTheme.typography.titleLarge
            )
            Spacer()

            when (viewModel.step) {
                LoginStep.CREDENTIALS -> CredentialsStep(viewModel)
                LoginStep.TWO_FACTOR -> TwoFactorStep(viewModel)
                LoginStep.DONE -> {
                    val blob = viewModel.sessionBlob
                    if (blob != null) onLoggedIn(blob)
                }
            }

            viewModel.errorMessage?.let { message ->
                Spacer()
                Text(
                    text = message,
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodyLarge
                )
            }

            if (viewModel.isLoading) {
                Spacer()
                CircularProgressIndicator()
            }
        }
    }
}

@Composable
private fun CredentialsStep(viewModel: LoginViewModel) {
    var appleId by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }

    OutlinedTextField(
        value = appleId,
        onValueChange = { appleId = it },
        label = { Text("Apple ID") },
        singleLine = true,
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Email),
        modifier = Modifier.fillMaxWidth(),
        enabled = !viewModel.isLoading
    )
    Spacer()
    OutlinedTextField(
        value = password,
        onValueChange = { password = it },
        label = { Text("Password") },
        singleLine = true,
        visualTransformation = PasswordVisualTransformation(),
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
        modifier = Modifier.fillMaxWidth(),
        enabled = !viewModel.isLoading
    )
    Spacer()
    Button(
        onClick = { viewModel.submitCredentials(appleId, password) },
        enabled = !viewModel.isLoading,
        modifier = Modifier.fillMaxWidth()
    ) {
        Text("Continue")
    }
}

@Composable
private fun TwoFactorStep(viewModel: LoginViewModel) {
    var code by remember { mutableStateOf("") }

    Text(
        text = "Enter the code sent to your trusted device",
        style = MaterialTheme.typography.bodyLarge
    )
    Spacer()
    OutlinedTextField(
        value = code,
        onValueChange = { if (it.length <= 6) code = it },
        label = { Text("6-digit code") },
        singleLine = true,
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.NumberPassword),
        modifier = Modifier.fillMaxWidth(),
        enabled = !viewModel.isLoading
    )
    Spacer()
    Button(
        onClick = { viewModel.submitTwoFactorCode(code) },
        enabled = !viewModel.isLoading,
        modifier = Modifier.fillMaxWidth()
    ) {
        Text("Verify")
    }
}

@Composable
private fun Spacer() {
    androidx.compose.foundation.layout.Spacer(modifier = Modifier.padding(8.dp))
}
