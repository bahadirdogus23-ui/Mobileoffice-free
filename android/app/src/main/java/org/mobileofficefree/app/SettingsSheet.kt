package org.mobileofficefree.app

import android.widget.Toast
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp

/** Privacy settings: app lock, screenshot block, incognito, and clearing the recent list. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsSheet(onDismiss: () -> Unit, onRecentsCleared: () -> Unit) {
    val context = LocalContext.current
    val privacy = remember { PrivacySettings.get(context) }
    val canLock = remember { AppLock().available(context) }
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Text(stringResource(R.string.settings), style = MaterialTheme.typography.titleLarge, modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp))
        Toggle(
            title = stringResource(R.string.settings_lock),
            hint = stringResource(if (canLock) R.string.settings_lock_hint else R.string.settings_lock_unavailable),
            checked = privacy.lockApp,
            enabled = canLock || privacy.lockApp,
            onChange = privacy::updateLockApp,
        )
        Toggle(stringResource(R.string.settings_secure), stringResource(R.string.settings_secure_hint), privacy.blockScreenshots, true, privacy::updateBlockScreenshots)
        Toggle(stringResource(R.string.settings_incognito), stringResource(R.string.settings_incognito_hint), privacy.incognito, true, privacy::updateIncognito)
        val cleared = stringResource(R.string.settings_recents_cleared)
        ListItem(
            headlineContent = { Text(stringResource(R.string.settings_clear_recents)) },
            supportingContent = { Text(stringResource(R.string.settings_clear_recents_hint)) },
            modifier = Modifier.clickable {
                RecentFiles(context).clear()
                onRecentsCleared()
                Toast.makeText(context, cleared, Toast.LENGTH_SHORT).show()
            },
        )
        Spacer(Modifier.height(24.dp))
    }
}

@Composable
private fun Toggle(title: String, hint: String, checked: Boolean, enabled: Boolean, onChange: (Boolean) -> Unit) {
    ListItem(
        headlineContent = { Text(title) },
        supportingContent = { Text(hint) },
        trailingContent = { Switch(checked = checked, onCheckedChange = onChange, enabled = enabled) },
        modifier = Modifier.clickable(enabled = enabled) { onChange(!checked) },
    )
}
