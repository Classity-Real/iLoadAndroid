package com.classityreal.iloadandroid.ui.theme

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext

private val FallbackLightScheme = lightColorScheme(
    primary = FallbackPrimaryLight,
    onPrimary = FallbackOnPrimaryLight,
    secondary = FallbackSecondaryLight,
    background = FallbackBackgroundLight,
    surface = FallbackSurfaceLight
)

private val FallbackDarkScheme = darkColorScheme(
    primary = FallbackPrimaryDark,
    onPrimary = FallbackOnPrimaryDark,
    secondary = FallbackSecondaryDark,
    background = FallbackBackgroundDark,
    surface = FallbackSurfaceDark
)

@Composable
fun ILoadAndroidTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    // Material You: pulls the scheme from the device wallpaper on Android 12+.
    // Turn off only for testing/screenshotting a fixed palette.
    dynamicColor: Boolean = true,
    content: @Composable () -> Unit
) {
    val context = LocalContext.current
    val colorScheme = when {
        dynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S ->
            if (darkTheme) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        darkTheme -> FallbackDarkScheme
        else -> FallbackLightScheme
    }

    MaterialTheme(
        colorScheme = colorScheme,
        typography = Typography,
        content = content
    )
}
