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
import androidx.compose.material3.Button
import androidx.compose.material3.FilledTonalButton
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.draw.clip
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.background
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
    const val DEVELOPER = "Bahadır Dögüş"
    const val EMAIL = "bahadirdogus23@gmail.com"

    /** A new e-mail to the developer in the person's mail app. */
    fun mail(context: Context) {
        val intent = Intent(Intent.ACTION_SENDTO, "mailto:$EMAIL".toUri())
            .putExtra(Intent.EXTRA_SUBJECT, "BadoOffice")
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        try {
            context.startActivity(intent)
        } catch (_: ActivityNotFoundException) {
            Toast.makeText(context, EMAIL, Toast.LENGTH_LONG).show()
        }
    }

    /** Donation pages: Bana Bi Coffee for Turkey (lira), Buy Me a Coffee from abroad. */
    const val COFFEE_TR_URL = "https://banabicoffee.com/@bado233"
    const val COFFEE_URL = "https://buymeacoffee.com/bado23"

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
                SupportChoice("☕", stringResource(R.string.buy_coffee)) { Coffee.open() }
                SupportChoice("📣", stringResource(R.string.support_share_app)) { Support.shareApp(context) }
                SupportChoice("⭐", stringResource(R.string.support_star)) { Support.open(context, Support.PROJECT_URL) }
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

/** "Buy me a coffee": a word on why, and both donation pages to choose from. */
object Coffee {
    var choosing by mutableStateOf(false)

    fun open() {
        choosing = true
    }
}

/** Shown after [Coffee.open]: why support matters, and the page for Turkey and the one for abroad. */
@Composable
fun CoffeeDialog() {
    if (!Coffee.choosing) return
    val context = LocalContext.current
    AlertDialog(
        onDismissRequest = { Coffee.choosing = false },
        icon = { Text("☕", fontSize = 34.sp) },
        title = { Text(stringResource(R.string.buy_coffee)) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                Text(stringResource(R.string.coffee_pitch), style = MaterialTheme.typography.bodyMedium)
                CoffeeChoice(stringResource(R.string.coffee_tr), stringResource(R.string.coffee_tr_hint), Color(0xFFE30A17)) {
                    Coffee.choosing = false
                    Support.open(context, Support.COFFEE_TR_URL)
                }
                CoffeeChoice(stringResource(R.string.coffee_global), stringResource(R.string.coffee_global_hint), Color(0xFFFFB300)) {
                    Coffee.choosing = false
                    Support.open(context, Support.COFFEE_URL)
                }
            }
        },
        confirmButton = { TextButton(onClick = { Coffee.choosing = false }) { Text(stringResource(R.string.coffee_later)) } },
    )
}

@Composable
private fun CoffeeChoice(title: String, hint: String, tint: Color, onClick: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().clip(RoundedCornerShape(14.dp)).background(tint.copy(alpha = 0.12f)).clickable(onClick = onClick).padding(12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text("☕", fontSize = 22.sp)
        Column(Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.titleSmall)
            Text(hint, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        Text("›", fontSize = 22.sp, color = tint)
    }
}

/** A quiet card for empty corners of the app: support and coffee, side by side. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun SupportCard(modifier: Modifier = Modifier) {
    var dialog by remember { mutableStateOf(false) }
    if (dialog) SupportDialog { dialog = false }
    CoffeeDialog()
    Card(
        modifier = modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerLow),
    ) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Text(stringResource(R.string.support_developer_hint), style = MaterialTheme.typography.bodySmall)
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                FilledTonalButton(onClick = { dialog = true }) { Text("♥ " + stringResource(R.string.support_developer)) }
                Button(onClick = { Coffee.open() }) { Text("☕ " + stringResource(R.string.buy_coffee)) }
            }
        }
    }
}
