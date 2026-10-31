package org.mobileofficefree.app

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.widget.Toast
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.net.toUri

/**
 * Ways to support the app. The app itself has no internet permission: links open in the
 * person's browser, and nothing is sent from here.
 */
object Support {
    const val PROJECT_URL = "https://github.com/bahadirdogus23-ui/Mobileoffice-free"
    const val ISSUES_URL = "$PROJECT_URL/issues"

    /** The donation page; empty until there is one (the person is told so instead). */
    const val COFFEE_URL = ""

    fun open(context: Context, url: String) {
        try {
            context.startActivity(Intent(Intent.ACTION_VIEW, url.toUri()).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        } catch (_: ActivityNotFoundException) {
            Toast.makeText(context, R.string.no_browser, Toast.LENGTH_SHORT).show()
        }
    }

    fun shareApp(context: Context) {
        val send = Intent(Intent.ACTION_SEND)
            .setType("text/plain")
            .putExtra(Intent.EXTRA_TEXT, context.getString(R.string.support_share_text, PROJECT_URL))
        context.startActivity(Intent.createChooser(send, context.getString(R.string.support_share_app)))
    }
}

/** What "Support the developer" offers. */
@Composable
fun SupportDialog(onDismiss: () -> Unit) {
    val context = LocalContext.current
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.support_developer)) },
        text = {
            Column {
                Text(stringResource(R.string.support_thanks), style = MaterialTheme.typography.bodyMedium)
                SupportChoice("☕", stringResource(R.string.buy_coffee)) { Coffee.open(context) }
                SupportChoice("📣", stringResource(R.string.support_share_app)) { Support.shareApp(context) }
                SupportChoice("⭐", stringResource(R.string.support_star)) { Support.open(context, Support.PROJECT_URL) }
                SupportChoice("✉", stringResource(R.string.support_report)) { Support.open(context, Support.ISSUES_URL) }
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.close)) } },
    )
}

@Composable
private fun SupportChoice(sign: String, label: String, onClick: () -> Unit) {
    ListItem(
        leadingContent = { Text(sign, fontSize = 20.sp) },
        headlineContent = { Text(label) },
        modifier = Modifier.clickable(onClick = onClick),
    )
}

/** "Buy me a coffee": the donation page, or a word that it is not ready yet. */
object Coffee {
    var notReady by mutableStateOf(false)

    fun open(context: Context) {
        if (Support.COFFEE_URL.isNotEmpty()) Support.open(context, Support.COFFEE_URL) else notReady = true
    }
}

/** Shown once [Coffee.open] found no donation page. */
@Composable
fun CoffeeNotReadyDialog() {
    if (!Coffee.notReady) return
    val context = LocalContext.current
    AlertDialog(
        onDismissRequest = { Coffee.notReady = false },
        title = { Text(stringResource(R.string.buy_coffee)) },
        text = { Text(stringResource(R.string.coffee_soon)) },
        confirmButton = {
            TextButton(onClick = {
                Coffee.notReady = false
                Support.shareApp(context)
            }) { Text(stringResource(R.string.support_share_app)) }
        },
        dismissButton = { TextButton(onClick = { Coffee.notReady = false }) { Text(stringResource(R.string.close)) } },
    )
}

/** A quiet card for empty corners of the app: support and coffee, side by side. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun SupportCard(modifier: Modifier = Modifier) {
    val context = LocalContext.current
    var dialog by remember { mutableStateOf(false) }
    if (dialog) SupportDialog { dialog = false }
    CoffeeNotReadyDialog()
    Card(
        modifier = modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.secondaryContainer.copy(alpha = 0.5f)),
    ) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Text(stringResource(R.string.support_developer_hint), style = MaterialTheme.typography.bodySmall)
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                TextButton(onClick = { dialog = true }) { Text("♥ " + stringResource(R.string.support_developer)) }
                TextButton(onClick = { Coffee.open(context) }) { Text("☕ " + stringResource(R.string.buy_coffee)) }
            }
        }
    }
}
