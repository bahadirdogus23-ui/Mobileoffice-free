package org.mobileofficefree.app

import android.widget.Toast
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.selection.toggleable
import androidx.compose.ui.semantics.Role
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.IconButton
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ListItem
import androidx.compose.material3.TextButton
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Slider
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import kotlin.math.roundToInt

/**
 * The settings screen: reading, privacy, support and about, each in its own card. Shown full
 * screen over the home screen.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsSheet(
    reading: ReadingSettings,
    engineVersion: String,
    onDiagnostics: () -> Unit,
    onDismiss: () -> Unit,
    onRecentsCleared: () -> Unit,
) {
    val context = LocalContext.current
    val privacy = remember { PrivacySettings.get(context) }
    val canLock = remember { AppLock().available(context) }
    val version = remember {
        runCatching { context.packageManager.getPackageInfo(context.packageName, 0).versionName }.getOrNull().orEmpty()
    }
    var support by remember { mutableStateOf(false) }
    var report by remember { mutableStateOf(false) }
    var editProfile by remember { mutableStateOf(false) }
    val profile = remember { Profile.get(context) }
    var guide by remember { mutableStateOf(false) }
    Dialog(onDismissRequest = onDismiss, properties = DialogProperties(usePlatformDefaultWidth = false)) {
        Scaffold(
            topBar = {
                TopAppBar(
                    title = { Text(stringResource(R.string.settings)) },
                    navigationIcon = { IconButton(onClick = onDismiss, modifier = Modifier.described(stringResource(R.string.back))) { Text("←", fontSize = 22.sp) } },
                )
            },
        ) { padding ->
            Column(
                Modifier.padding(padding).fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 16.dp),
            ) {
                Section(stringResource(R.string.settings_section_look)) { AppearanceSettings() }
                Section(stringResource(R.string.settings_section_reading)) {
                    Toggle("☾", Color(0xFF3949AB), stringResource(R.string.night_mode), stringResource(R.string.night_mode_hint), reading.night, true, reading::updateNight)
                    if (reading.night) {
                        Toggle("◑", Color(0xFF283593), stringResource(R.string.dark_pages), stringResource(R.string.dark_pages_hint), reading.darkPages, true, reading::updateDarkPages)
                    }
                    Toggle("☀", Color(0xFFF9A825), stringResource(R.string.keep_screen_on), null, reading.keepScreenOn, true, reading::updateKeepScreenOn)
                    Toggle("⇅", Color(0xFF00897B), stringResource(R.string.volume_keys), stringResource(R.string.volume_keys_hint), reading.volumeKeys, true, reading::updateVolumeKeys)
                    Item(
                        "Aa",
                        Color(0xFF6D4C41),
                        stringResource(R.string.settings_text_size, reading.textSize.roundToInt()),
                        support = {
                            Slider(value = reading.textSize, onValueChange = reading::updateTextSize, valueRange = 12f..40f, steps = 13)
                        },
                    )
                }
                Section(stringResource(R.string.settings_section_privacy)) {
                    Toggle(
                        "🔒",
                        Color(0xFF455A64),
                        stringResource(R.string.settings_lock),
                        stringResource(if (canLock) R.string.settings_lock_hint else R.string.settings_lock_unavailable),
                        privacy.lockApp,
                        canLock || privacy.lockApp,
                        privacy::updateLockApp,
                    )
                    Toggle("⛶", Color(0xFF5E35B1), stringResource(R.string.settings_secure), stringResource(R.string.settings_secure_hint), privacy.blockScreenshots, true, privacy::updateBlockScreenshots)
                    Toggle("◐", Color(0xFF546E7A), stringResource(R.string.settings_incognito), stringResource(R.string.settings_incognito_hint), privacy.incognito, true, privacy::updateIncognito)
                    val cleared = stringResource(R.string.settings_recents_cleared)
                    Item("⌫", Color(0xFFC62828), stringResource(R.string.settings_clear_recents), stringResource(R.string.settings_clear_recents_hint)) {
                        RecentFiles(context).clear()
                        onRecentsCleared()
                        Toast.makeText(context, cleared, Toast.LENGTH_SHORT).show()
                    }
                }
                Section(stringResource(R.string.settings_profile)) {
                    Item("👤", Color(0xFF0A84FF), profile.name.ifBlank { stringResource(R.string.profile_unset) }, stringResource(R.string.settings_profile_hint)) { editProfile = true }
                }
                Section(stringResource(R.string.settings_section_support)) {
                    Item("♥", Color(0xFFD81B60), stringResource(R.string.support_developer), stringResource(R.string.support_developer_hint)) { support = true }
                    Item("☕", Color(0xFF795548), stringResource(R.string.buy_coffee), stringResource(R.string.buy_coffee_hint)) { Coffee.open() }
                    Item("📣", Color(0xFF1E88E5), stringResource(R.string.support_share_app), null) { Support.shareApp(context) }
                    // One place to report a problem or suggest something: by email or on GitHub.
                    Item("🐞", Color(0xFFE53935), stringResource(R.string.support_report), stringResource(R.string.report_problem_hint)) { report = true }
                    val reports = remember { ReportSettings.get(context) }
                    Toggle("📷", Color(0xFF6D4C41), stringResource(R.string.report_screenshots), stringResource(R.string.report_screenshots_hint), reports.screenshots, true, reports::updateScreenshots)
                }
                Section(stringResource(R.string.settings_section_about)) {
                    Item("?", Color(0xFF00897B), stringResource(R.string.guide), stringResource(R.string.guide_hint)) { guide = true }
                    Item("👤", Color(0xFF5856D6), stringResource(R.string.about_developer), Support.DEVELOPER)
                    Item("✉", Color(0xFF0A84FF), stringResource(R.string.about_contact), Support.EMAIL) { Support.mail(context) }
                    Item("ℹ", Color(0xFF757575), stringResource(R.string.about_version), version)
                    Item("⚙", Color(0xFF757575), stringResource(R.string.about_engines), engineVersion)
                    Item("⏱", Color(0xFF757575), stringResource(R.string.diagnostics), stringResource(R.string.about_diagnostics_hint)) {
                        onDismiss()
                        onDiagnostics()
                    }
                }
                Section(stringResource(R.string.about_purpose)) {
                    Text(
                        stringResource(R.string.about_purpose_text),
                        style = MaterialTheme.typography.bodyMedium,
                        modifier = Modifier.padding(16.dp),
                    )
                }
                Text(
                    stringResource(R.string.about_promise),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.fillMaxWidth().padding(vertical = 20.dp),
                )
            }
        }
    }
    // Composed after the settings screen, so they open over it.
    if (support) SupportDialog { support = false }
    if (editProfile) {
        var name by remember { mutableStateOf(profile.name) }
        AlertDialog(
            onDismissRequest = { editProfile = false },
            title = { Text(stringResource(R.string.settings_profile)) },
            text = {
                androidx.compose.material3.OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    singleLine = true,
                    label = { Text(stringResource(R.string.profile_name)) },
                )
            },
            confirmButton = {
                TextButton(onClick = {
                    profile.updateName(name)
                    editProfile = false
                }) { Text(stringResource(R.string.save)) }
            },
            dismissButton = { TextButton(onClick = { editProfile = false }) { Text(stringResource(R.string.cancel)) } },
        )
    }
    if (report) {
        AlertDialog(
            onDismissRequest = { report = false },
            title = { Text(stringResource(R.string.support_report)) },
            text = {
                Column {
                    ListItem(
                        leadingContent = { Text("✉", fontSize = 20.sp, modifier = Modifier.decorative()) },
                        headlineContent = { Text(stringResource(R.string.report_by_email)) },
                        modifier = Modifier.clickable {
                            report = false
                            Diagnostics.emailReport(context, "Hata / öneri")
                        },
                    )
                    ListItem(
                        leadingContent = { Text("🐙", fontSize = 20.sp, modifier = Modifier.decorative()) },
                        headlineContent = { Text(stringResource(R.string.report_on_github)) },
                        modifier = Modifier.clickable {
                            report = false
                            Support.open(context, Support.ISSUES_URL)
                        },
                    )
                }
            },
            confirmButton = { TextButton(onClick = { report = false }) { Text(stringResource(R.string.cancel)) } },
        )
    }
    if (guide) GuideScreen { guide = false }
    CoffeeDialog()
}

/** A titled card holding rows, separated by thin lines. */
@Composable
private fun Section(title: String, rows: @Composable () -> Unit) {
    Text(
        title.uppercase(java.util.Locale.forLanguageTag("tr")),
        style = MaterialTheme.typography.labelSmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(start = 16.dp, top = 24.dp, bottom = 6.dp),
    )
    Card(
        shape = MaterialTheme.shapes.large,
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerLow),
    ) {
        Column { rows() }
    }
}

/** The sign of a row: [sign] on a rounded tile of [tint]. */
@Composable
private fun Tile(sign: String, tint: Color) {
    Box(Modifier.size(36.dp).clip(RoundedCornerShape(10.dp)).background(tint), contentAlignment = Alignment.Center) {
        Text(sign, color = Color.White, fontSize = with(LocalDensity.current) { 17.dp.toSp() }, modifier = Modifier.decorative())
    }
}

@Composable
private fun Item(sign: String, tint: Color, title: String, hint: String? = null, support: (@Composable () -> Unit)? = null, onClick: (() -> Unit)? = null) {
    ListItem(
        leadingContent = { Tile(sign, tint) },
        headlineContent = { Text(title) },
        supportingContent = support ?: hint?.let { { Text(it) } },
        colors = ListItemDefaults.colors(containerColor = Color.Transparent),
        modifier = if (onClick != null) Modifier.clickable(onClick = onClick) else Modifier,
    )
}

@Composable
private fun Toggle(sign: String, tint: Color, title: String, hint: String?, checked: Boolean, enabled: Boolean, onChange: (Boolean) -> Unit) {
    ListItem(
        leadingContent = { Tile(sign, tint) },
        headlineContent = { Text(title) },
        supportingContent = hint?.let { { Text(it) } },
        trailingContent = { Switch(checked = checked, onCheckedChange = null, enabled = enabled) },
        colors = ListItemDefaults.colors(containerColor = Color.Transparent),
        modifier = Modifier.toggleable(value = checked, enabled = enabled, role = Role.Switch, onValueChange = onChange),
    )
}
