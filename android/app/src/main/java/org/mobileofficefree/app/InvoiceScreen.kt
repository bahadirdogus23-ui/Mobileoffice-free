package org.mobileofficefree.app

import android.content.Intent
import android.print.PrintAttributes
import android.print.PrintManager
import android.webkit.WebView
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.BottomAppBar
import androidx.compose.material3.Card
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
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView

/**
 * An e-invoice: a summary card (who, when, how much) over the document itself, drawn by a
 * WebView with JavaScript, network and file access all off: the HTML comes from the file.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun InvoiceScreen(vm: InvoiceViewModel, state: InvoiceUiState) {
    val context = LocalContext.current
    val s = state.summary
    var showSummary by remember(state.uri) { mutableStateOf(true) }
    var web by remember { mutableStateOf<WebView?>(null) }
    BackHandler { vm.close() }

    Column(Modifier.fillMaxSize()) {
        TopAppBar(
            title = {
                Column {
                    Text(Invoices.title(s), maxLines = 1)
                    Text(s.number, style = MaterialTheme.typography.bodySmall, maxLines = 1)
                }
            },
            navigationIcon = {
                IconButton(onClick = vm::close) {
                    Icon(painterResource(R.drawable.ic_arrow_back), contentDescription = stringResource(R.string.back))
                }
            },
        )
        if (showSummary) {
            Card(Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 6.dp)) {
                SelectionContainer {
                    Column(Modifier.padding(12.dp)) {
                        Text(s.payable.ifEmpty { "—" }, style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.SemiBold)
                        Text(stringResource(R.string.invoice_payable), style = MaterialTheme.typography.labelSmall)
                        Line(stringResource(R.string.invoice_seller), party(s.supplier))
                        Line(stringResource(R.string.invoice_buyer), party(s.customer))
                        Line(stringResource(R.string.invoice_date), s.date)
                        if (s.taxTotal.isNotEmpty()) Line(stringResource(R.string.invoice_vat), "${s.taxTotal}  (${stringResource(R.string.invoice_base)} ${s.taxExclusive})")
                        if (s.uuid.isNotEmpty()) Line("ETTN", s.uuid)
                        Text(
                            stringResource(if (state.ownDesign) R.string.invoice_own_design else R.string.invoice_plain_design),
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.padding(top = 6.dp),
                        )
                    }
                }
            }
        }
        AndroidView(
            factory = { ctx ->
                WebView(ctx).apply {
                    settings.javaScriptEnabled = false
                    settings.blockNetworkLoads = true
                    settings.allowFileAccess = false
                    settings.allowContentAccess = false
                    settings.builtInZoomControls = true
                    settings.displayZoomControls = false
                    // Invoice designs are laid out for A4: show the whole width, zoom in to read.
                    settings.useWideViewPort = true
                    settings.loadWithOverviewMode = true
                    web = this
                }
            },
            update = {
                // Load once per document, not on every recomposition (e.g. hiding the summary).
                if (it.tag != state.html) {
                    it.tag = state.html
                    it.loadDataWithBaseURL(null, state.html, "text/html", "utf-8", null)
                }
            },
            modifier = Modifier.weight(1f).fillMaxWidth(),
        )
        BottomAppBar {
            TextButton(onClick = { showSummary = !showSummary }) { Text(stringResource(R.string.invoice_summary)) }
            TextButton(onClick = {
                val w = web ?: return@TextButton
                val name = "${Invoices.title(s)} ${s.number}".trim()
                val manager = context.getSystemService(PrintManager::class.java) ?: return@TextButton
                manager.print(name, w.createPrintDocumentAdapter(name), PrintAttributes.Builder().setMediaSize(PrintAttributes.MediaSize.ISO_A4).build())
            }) { Text(stringResource(R.string.print)) }
            TextButton(onClick = {
                val send = Intent(Intent.ACTION_SEND)
                    .setType("application/xml")
                    .putExtra(Intent.EXTRA_STREAM, state.uri)
                    .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                context.startActivity(Intent.createChooser(send, state.fileName))
            }) { Text(stringResource(R.string.share)) }
        }
    }
}

private fun party(p: InvoiceParty): String =
    listOf(p.name, if (p.taxId.isNotEmpty()) "${p.taxIdScheme.ifEmpty { "VKN" }} ${p.taxId}" else "").filter { it.isNotEmpty() }.joinToString(" · ")

@Composable
private fun Line(label: String, value: String) {
    if (value.isEmpty()) return
    Row(Modifier.padding(top = 4.dp)) {
        Text("$label: ", style = MaterialTheme.typography.bodySmall, fontWeight = FontWeight.SemiBold)
        Text(value, style = MaterialTheme.typography.bodySmall, maxLines = 2, overflow = TextOverflow.Ellipsis)
    }
}
