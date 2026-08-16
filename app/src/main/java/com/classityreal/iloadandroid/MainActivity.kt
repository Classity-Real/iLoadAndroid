package com.classityreal.iloadandroid

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.classityreal.iloadandroid.root.RootManager
import com.classityreal.iloadandroid.ui.LoginScreen
import com.classityreal.iloadandroid.ui.theme.ILoadAndroidTheme
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            ILoadAndroidTheme {
                Surface(color = MaterialTheme.colorScheme.background) {
                    AppRoot()
                }
            }
        }
    }
}

private sealed class RootGateState {
    data object Checking : RootGateState()
    data object Denied : RootGateState()
    data object Granted : RootGateState()
}

@Composable
private fun AppRoot() {
    var gateState by remember { mutableStateOf<RootGateState>(RootGateState.Checking) }
    val scope = rememberCoroutineScopeCompat()

    remember {
        scope.launch {
            gateState = when (RootManager.checkRoot()) {
                RootManager.RootStatus.Granted -> RootGateState.Granted
                else -> RootGateState.Denied
            }
        }
        true
    }

    when (gateState) {
        is RootGateState.Checking -> LoadingScreen("Checking for root access…")
        is RootGateState.Denied -> RootRequiredScreen()
        is RootGateState.Granted -> LoginScreen(onLoggedIn = { sessionBlob ->
            // TODO: hand off to Android Keystore-backed storage, then
            // navigate into the device/sideload phase once that's built.
        })
    }
}

@Composable
private fun LoadingScreen(message: String) {
    Scaffold { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding).padding(24.dp),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            CircularProgressIndicator()
            Text(text = message, modifier = Modifier.padding(top = 16.dp))
        }
    }
}

@Composable
private fun RootRequiredScreen() {
    Scaffold { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding).padding(24.dp),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Text(
                text = "iLoadAndroid needs root access to manage device pairing and app installation.",
                style = MaterialTheme.typography.bodyLarge
            )
        }
    }
}

@Composable
private fun rememberCoroutineScopeCompat() = androidx.compose.runtime.rememberCoroutineScope()
