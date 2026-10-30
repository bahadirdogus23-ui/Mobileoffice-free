package org.mobileofficefree.app

import android.app.Application
import android.net.Uri
import android.os.SystemClock
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

data class InvoiceUiState(
    val uri: Uri,
    val fileName: String,
    val summary: InvoiceSummary,
    val html: String,
    /** The seller's embedded XSLT design was used (else the app's plain layout). */
    val ownDesign: Boolean,
)

/** GİB e-Fatura / e-Arşiv / e-İrsaliye XML: a summary and the document drawn as HTML. */
class InvoiceViewModel(app: Application) : AndroidViewModel(app) {
    private val recents = RecentFiles(app)

    var state by mutableStateOf<InvoiceUiState?>(null)
        private set
    var loading by mutableStateOf(false)
        private set
    var error by mutableStateOf<String?>(null)
        private set

    fun open(uri: Uri) {
        val resolver = getApplication<Application>().contentResolver
        loading = true
        val started = SystemClock.elapsedRealtime()
        viewModelScope.launch {
            val result = withContext(Dispatchers.IO) {
                runCatching {
                    val info = FileInfo.of(resolver, uri)
                    val bytes = resolver.openInputStream(uri)?.use { it.readBytes() } ?: error("no data")
                    val summary = Invoices.read(bytes)
                    val (html, own) = Invoices.html(bytes, summary)
                    recents.remember(uri, info.name, DocumentKind.INVOICE, info.size)
                    Diagnostics.record("INVOICE açma (${sizeLabel(info.size)}, ${if (own) "XSLT" else "sade"})", started)
                    InvoiceUiState(uri, info.name, summary, html, own)
                }
            }
            loading = false
            result.fold(onSuccess = { state = it }, onFailure = { error = it.message ?: it.javaClass.simpleName })
        }
    }

    fun close() {
        state = null
    }

    fun consumeError() {
        error = null
    }
}
