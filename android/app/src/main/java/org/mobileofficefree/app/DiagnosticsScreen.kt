package org.mobileofficefree.app

import android.content.Intent
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.BottomAppBar
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.foundation.horizontalScroll
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp

/** Phone, size, memory and opening times, to read off or share with the developers. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DiagnosticsScreen(onBack: () -> Unit) {
    val context = LocalContext.current
    var refresh by remember { mutableIntStateOf(0) }
    val report = remember(refresh) { Diagnostics.report(context) }
    var askClear by remember { mutableStateOf(false) }
    if (askClear) {
        androidx.compose.material3.AlertDialog(
            onDismissRequest = { askClear = false },
            text = { Text(stringResource(R.string.diagnostics_clear_confirm)) },
            confirmButton = {
                TextButton(onClick = {
                    askClear = false
                    Diagnostics.clear()
                    refresh++
                }) { Text(stringResource(R.string.diagnostics_clear)) }
            },
            dismissButton = { TextButton(onClick = { askClear = false }) { Text(stringResource(R.string.cancel)) } },
        )
    }
    BackHandler(onBack = onBack)
    Column(Modifier.fillMaxSize()) {
        TopAppBar(
            title = { Text(stringResource(R.string.diagnostics)) },
            navigationIcon = {
                IconButton(onClick = onBack) {
                    Icon(painterResource(R.drawable.ic_arrow_back), contentDescription = stringResource(R.string.back))
                }
            },
        )
        Text(
            stringResource(R.string.diagnostics_hint),
            style = MaterialTheme.typography.bodySmall,
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp),
        )
        SelectionContainer(Modifier.weight(1f).verticalScroll(rememberScrollState())) {
            Text(report, fontFamily = FontFamily.Monospace, style = MaterialTheme.typography.bodySmall, modifier = Modifier.padding(16.dp))
        }
        BottomAppBar {
            androidx.compose.foundation.layout.Row(Modifier.horizontalScroll(rememberScrollState())) {
            TextButton(onClick = { refresh++ }) { Text(stringResource(R.string.refresh)) }
            TextButton(onClick = { shareText(context, report) }) { Text(stringResource(R.string.share)) }
            TextButton(onClick = { Diagnostics.emailReport(context, "Ölçümler") }) { Text(stringResource(R.string.report_email)) }
            TextButton(onClick = { askClear = true }) { Text(stringResource(R.string.diagnostics_clear)) }
            }
        }
    }
}

/** Hands [text] to any app the person picks (mail, messaging); the app itself never goes online. */
fun shareText(context: android.content.Context, text: String) {
    val send = Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, text)
    context.startActivity(Intent.createChooser(send, null))
}
