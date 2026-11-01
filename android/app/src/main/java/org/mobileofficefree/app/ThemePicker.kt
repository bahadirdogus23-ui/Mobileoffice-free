package org.mobileofficefree.app

import android.os.Build
import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp

/**
 * iPhone-style segmented control: equal segments on a grey track, the chosen one raised in white.
 */
@Composable
fun Segmented(options: List<String>, selected: Int, onSelect: (Int) -> Unit, modifier: Modifier = Modifier) {
    val colors = MaterialTheme.colorScheme
    Row(
        modifier.fillMaxWidth().clip(RoundedCornerShape(10.dp)).background(colors.surfaceVariant).padding(3.dp),
        horizontalArrangement = Arrangement.spacedBy(3.dp),
    ) {
        options.forEachIndexed { i, label ->
            val bg by animateColorAsState(if (i == selected) colors.surfaceContainerHigh else Color.Transparent, label = "segment")
            Box(
                Modifier.weight(1f).clip(RoundedCornerShape(8.dp)).background(bg).clickable { onSelect(i) }.padding(vertical = 7.dp),
                contentAlignment = Alignment.Center,
            ) {
                Text(label, style = MaterialTheme.typography.labelLarge, color = colors.onSurface, maxLines = 1, textAlign = TextAlign.Center)
            }
        }
    }
}

/** Light/dark, the look (presets, the phone's colours, custom), and the custom look's settings. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun AppearanceSettings() {
    val context = LocalContext.current
    val theme = remember { ThemeSettings.get(context) }
    val colors = MaterialTheme.colorScheme
    Column(Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
        Label(R.string.theme_mode)
        Segmented(ThemeMode.entries.map { stringResource(it.label) }, theme.mode.ordinal, { theme.updateMode(ThemeMode.entries[it]) })

        Label(R.string.theme_look)
        FlowRow(horizontalArrangement = Arrangement.spacedBy(14.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            for (p in ThemePreset.entries) {
                Swatch(Brush.linearGradient(listOf(Color(p.accent), Color(p.accent).copy(alpha = 0.55f))), stringResource(p.label), theme.look == p.name) {
                    theme.updateLook(p.name)
                }
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                Swatch(
                    Brush.sweepGradient(listOf(Color(0xFF4285F4), Color(0xFF34A853), Color(0xFFFBBC05), Color(0xFFEA4335), Color(0xFF4285F4))),
                    stringResource(R.string.theme_phone),
                    theme.look == ThemeSettings.PHONE,
                ) { theme.updateLook(ThemeSettings.PHONE) }
            }
            Swatch(
                Brush.sweepGradient(listOf(Color.Red, Color.Yellow, Color.Green, Color.Cyan, Color.Blue, Color.Magenta, Color.Red)),
                stringResource(R.string.theme_custom),
                theme.look == ThemeSettings.CUSTOM,
            ) { theme.updateLook(ThemeSettings.CUSTOM) }
        }

        if (theme.look == ThemeSettings.CUSTOM) {
            // Create your own: any hue for the accent, how lively it is, and a tint for the background.
            Text(stringResource(R.string.theme_create), style = MaterialTheme.typography.titleSmall, color = colors.primary)
            val hsv = remember(theme.customAccent) { FloatArray(3).also { android.graphics.Color.colorToHSV(theme.customAccent, it) } }
            Label(R.string.theme_hue)
            Box(Modifier.fillMaxWidth().height(10.dp).clip(CircleShape).background(Brush.horizontalGradient((0..6).map { Color.hsv(it * 60f % 360f, 0.8f, 0.95f) })))
            Slider(
                value = hsv[0],
                onValueChange = { h -> theme.updateCustomAccent(Color.hsv(h, hsv[1].coerceAtLeast(0.35f), hsv[2].coerceAtLeast(0.55f)).toArgb()) },
                valueRange = 0f..359f,
                colors = SliderDefaults.colors(thumbColor = Color(theme.customAccent), activeTrackColor = Color(theme.customAccent)),
            )
            Label(R.string.theme_accent)
            Slider(
                value = hsv[1],
                onValueChange = { s -> theme.updateCustomAccent(Color.hsv(hsv[0], s, hsv[2].coerceAtLeast(0.55f)).toArgb()) },
                valueRange = 0.25f..1f,
                colors = SliderDefaults.colors(thumbColor = Color(theme.customAccent), activeTrackColor = Color(theme.customAccent)),
            )
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(stringResource(R.string.theme_tint), style = MaterialTheme.typography.bodyLarge, modifier = Modifier.weight(1f))
                Switch(checked = theme.customTint != 0, onCheckedChange = { theme.updateCustomTint(if (it) theme.customAccent else 0) })
            }
        }

        Label(R.string.theme_round)
        Segmented(listOf(R.string.theme_round_0, R.string.theme_round_1, R.string.theme_round_2).map { stringResource(it) }, theme.roundness, theme::updateRoundness)
    }
}

@Composable
private fun Label(text: Int) {
    Text(stringResource(text).uppercase(java.util.Locale.forLanguageTag("tr")), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
}

/** A round sample of a look with its name; a ring marks the chosen one. */
@Composable
private fun Swatch(fill: Brush, name: String, chosen: Boolean, onClick: () -> Unit) {
    val ring = MaterialTheme.colorScheme.onSurface
    Column(horizontalAlignment = Alignment.CenterHorizontally, modifier = Modifier.width(64.dp).clickable(onClick = onClick)) {
        Box(
            Modifier.size(46.dp).border(if (chosen) 2.5.dp else 0.dp, if (chosen) ring else Color.Transparent, CircleShape).padding(4.dp)
                .clip(CircleShape).background(fill),
        )
        Text(name, style = MaterialTheme.typography.labelSmall, maxLines = 1, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}
