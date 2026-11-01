package org.mobileofficefree.app

import android.content.Context
import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.edit

/** Light, dark, or as the phone is set. */
enum class ThemeMode(val label: Int) { SYSTEM(R.string.theme_system), LIGHT(R.string.theme_light), DARK(R.string.theme_dark) }

/** The ready-made looks: a lively accent each, on soft grouped backgrounds. */
enum class ThemePreset(val label: Int, val accent: Long) {
    OCEAN(R.string.theme_ocean, 0xFF0A84FF),
    MINT(R.string.theme_mint, 0xFF00C7A3),
    LAVENDER(R.string.theme_lavender, 0xFF7D6BFF),
    CORAL(R.string.theme_coral, 0xFFFF6F61),
    SUNSET(R.string.theme_sunset, 0xFFFF9F0A),
    ROSE(R.string.theme_rose, 0xFFFF375F),
    FOREST(R.string.theme_forest, 0xFF34A853),
    GRAPHITE(R.string.theme_graphite, 0xFF5E6B7A),
}

/** What the person chose for the app's look, kept across launches. */
class ThemeSettings private constructor(context: Context) {
    private val prefs = context.applicationContext.getSharedPreferences("theme", Context.MODE_PRIVATE)

    var mode by mutableStateOf(ThemeMode.entries.firstOrNull { it.name == prefs.getString(MODE, null) } ?: ThemeMode.SYSTEM)
        private set
    /** A preset's name, [PHONE] (the phone's own colours, Android 12+) or [CUSTOM]. */
    var look by mutableStateOf(prefs.getString(LOOK, null) ?: ThemePreset.OCEAN.name)
        private set
    /** The accent of the custom look, ARGB. */
    var customAccent by mutableIntStateOf(prefs.getInt(CUSTOM_ACCENT, 0xFF0A84FF.toInt()))
        private set
    /** The custom look's background tint, ARGB; 0 = neutral. */
    var customTint by mutableIntStateOf(prefs.getInt(CUSTOM_TINT, 0))
        private set
    /** Corner roundness: 0 square-ish, 1 soft, 2 round. */
    var roundness by mutableIntStateOf(prefs.getInt(ROUNDNESS, 1))
        private set

    fun updateMode(m: ThemeMode) { mode = m; prefs.edit { putString(MODE, m.name) } }
    fun updateLook(l: String) { look = l; prefs.edit { putString(LOOK, l) } }
    fun updateCustomAccent(c: Int) { customAccent = c; look = CUSTOM; prefs.edit { putInt(CUSTOM_ACCENT, c).putString(LOOK, CUSTOM) } }
    fun updateCustomTint(c: Int) { customTint = c; look = CUSTOM; prefs.edit { putInt(CUSTOM_TINT, c).putString(LOOK, CUSTOM) } }
    fun updateRoundness(r: Int) { roundness = r.coerceIn(0, 2); prefs.edit { putInt(ROUNDNESS, roundness) } }

    companion object {
        const val PHONE = "PHONE"
        const val CUSTOM = "CUSTOM"
        private const val MODE = "mode"
        private const val LOOK = "look"
        private const val CUSTOM_ACCENT = "custom_accent"
        private const val CUSTOM_TINT = "custom_tint"
        private const val ROUNDNESS = "roundness"

        @Volatile private var instance: ThemeSettings? = null
        fun get(context: Context): ThemeSettings = instance ?: synchronized(this) { instance ?: ThemeSettings(context).also { instance = it } }
    }
}

/**
 * A colour scheme in the spirit of iPhone settings: grouped light-grey (or black) backgrounds,
 * white (or dark grey) cards, clear text, and one lively [accent] for what can be touched.
 * [tint] colours the backgrounds very slightly (custom looks).
 */
fun softScheme(accent: Color, dark: Boolean, tint: Color? = null): ColorScheme {
    fun bg(base: Color, amount: Float) = if (tint == null) base else lerp(base, tint, amount)
    return if (!dark) {
        val onAccent = if (accent.luminance() > 0.6f) Color(0xFF1C1C1E) else Color.White
        lightColorScheme(
            primary = accent,
            onPrimary = onAccent,
            primaryContainer = lerp(accent, Color.White, 0.84f),
            onPrimaryContainer = lerp(accent, Color.Black, 0.55f),
            secondary = lerp(accent, Color(0xFF6C6C70), 0.45f),
            onSecondary = Color.White,
            secondaryContainer = lerp(accent, Color.White, 0.9f),
            onSecondaryContainer = lerp(accent, Color.Black, 0.6f),
            tertiary = lerp(accent, Color(0xFFFF2D55), 0.5f),
            background = bg(Color(0xFFF2F2F7), 0.06f),
            onBackground = Color(0xFF1C1C1E),
            surface = bg(Color(0xFFF2F2F7), 0.06f),
            onSurface = Color(0xFF1C1C1E),
            surfaceVariant = bg(Color(0xFFE9E9EE), 0.08f),
            onSurfaceVariant = Color(0xFF6C6C70),
            surfaceContainerLowest = Color.White,
            surfaceContainerLow = Color.White,
            surfaceContainer = bg(Color(0xFFFAFAFC), 0.04f),
            surfaceContainerHigh = Color.White,
            surfaceContainerHighest = bg(Color(0xFFE5E5EA), 0.08f),
            outline = Color(0xFFC6C6C8),
            outlineVariant = Color(0xFFE3E3E8),
            error = Color(0xFFFF3B30),
        )
    } else {
        val bright = lerp(accent, Color.White, 0.12f)
        darkColorScheme(
            primary = bright,
            onPrimary = if (bright.luminance() > 0.5f) Color(0xFF1C1C1E) else Color.White,
            primaryContainer = lerp(accent, Color.Black, 0.55f),
            onPrimaryContainer = lerp(accent, Color.White, 0.75f),
            secondary = lerp(accent, Color(0xFFAEAEB2), 0.5f),
            onSecondary = Color.Black,
            secondaryContainer = lerp(accent, Color(0xFF1C1C1E), 0.7f),
            onSecondaryContainer = lerp(accent, Color.White, 0.8f),
            tertiary = lerp(accent, Color(0xFFFF375F), 0.5f),
            background = bg(Color.Black, 0.05f),
            onBackground = Color.White,
            surface = bg(Color.Black, 0.05f),
            onSurface = Color.White,
            surfaceVariant = bg(Color(0xFF2C2C2E), 0.06f),
            onSurfaceVariant = Color(0xFFAEAEB2),
            surfaceContainerLowest = Color(0xFF0E0E10),
            surfaceContainerLow = bg(Color(0xFF1C1C1E), 0.06f),
            surfaceContainer = bg(Color(0xFF1C1C1E), 0.06f),
            surfaceContainerHigh = bg(Color(0xFF2C2C2E), 0.06f),
            surfaceContainerHighest = bg(Color(0xFF3A3A3C), 0.06f),
            outline = Color(0xFF545458),
            outlineVariant = Color(0xFF38383A),
            error = Color(0xFFFF453A),
        )
    }
}

/** Rounded corners for the chosen roundness (0, 1 or 2). */
fun softShapes(roundness: Int): Shapes {
    val k = when (roundness) { 0 -> 0.5f; 2 -> 1.5f; else -> 1f }
    return Shapes(
        extraSmall = RoundedCornerShape((6 * k).dp),
        small = RoundedCornerShape((10 * k).dp),
        medium = RoundedCornerShape((14 * k).dp),
        large = RoundedCornerShape((20 * k).dp),
        extraLarge = RoundedCornerShape((28 * k).dp),
    )
}

/** Large bold titles, as iPhone screens have them; the rest Material's. */
private val softTypography = Typography().let { t ->
    t.copy(
        headlineMedium = t.headlineMedium.copy(fontWeight = FontWeight.Bold, letterSpacing = (-0.5).sp),
        headlineSmall = t.headlineSmall.copy(fontWeight = FontWeight.Bold),
        titleLarge = t.titleLarge.copy(fontWeight = FontWeight.SemiBold),
        titleMedium = t.titleMedium.copy(fontWeight = FontWeight.SemiBold),
        labelSmall = TextStyle(fontSize = 11.sp, fontWeight = FontWeight.Medium, letterSpacing = 0.3.sp),
    )
}

@Composable
fun MobileOfficeTheme(content: @Composable () -> Unit) {
    val context = LocalContext.current
    val settings = remember { ThemeSettings.get(context) }
    val dark = when (settings.mode) {
        ThemeMode.SYSTEM -> isSystemInDarkTheme()
        ThemeMode.LIGHT -> false
        ThemeMode.DARK -> true
    }
    val colors = when (val look = settings.look) {
        ThemeSettings.PHONE -> if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            if (dark) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        } else {
            softScheme(Color(ThemePreset.OCEAN.accent), dark)
        }
        ThemeSettings.CUSTOM -> softScheme(Color(settings.customAccent), dark, settings.customTint.takeIf { it != 0 }?.let { Color(it) })
        else -> softScheme(Color((ThemePreset.entries.firstOrNull { it.name == look } ?: ThemePreset.OCEAN).accent), dark)
    }
    MaterialTheme(colorScheme = colors, shapes = softShapes(settings.roundness), typography = softTypography, content = content)
}
