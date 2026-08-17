package com.classityreal.iloadandroid.root

import com.topjohnwu.superuser.Shell
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * Thin wrapper around libsu. The auth/login phase doesn't actually need
 * root (Apple ID login is pure network) — this exists so the app can
 * surface "root required" up front, and so later phases (running a
 * usbmuxd-equivalent daemon, raw /dev/bus/usb access) have a single place
 * to request a root shell from.
 */
object RootManager {

    init {
        Shell.enableVerboseLogging = false
        // Fails fast rather than hanging indefinitely on a broken su binary.
        Shell.setDefaultBuilder(
            Shell.Builder.create().setTimeout(10)
        )
    }

    sealed class RootStatus {
        data object Granted : RootStatus()
        data object Denied : RootStatus()
        data object Unavailable : RootStatus()
    }

    /** Triggers the su prompt (if a root manager like Magisk is installed) and waits for the result. */
    suspend fun checkRoot(): RootStatus = withContext(Dispatchers.IO) {
        try {
            val shell = Shell.getShell()
            when {
                shell.isRoot -> RootStatus.Granted
                else -> RootStatus.Denied
            }
        } catch (_: Exception) {
            RootStatus.Unavailable
        }
    }

    /** Runs a single privileged command, returning stdout lines. Caller decides how to interpret failure. */
    suspend fun runAsRoot(command: String): List<String> = withContext(Dispatchers.IO) {
        Shell.cmd(command).exec().out
    }
}
