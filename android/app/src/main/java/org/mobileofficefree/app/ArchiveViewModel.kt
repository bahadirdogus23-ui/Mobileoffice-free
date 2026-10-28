package org.mobileofficefree.app

import android.app.Application
import android.content.Context
import android.net.Uri
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.core.content.FileProvider
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import java.io.File
import java.nio.charset.Charset
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** Where unpacked archive files live, and the provider that hands them to the viewers. */
object ArchiveFiles {
    fun authority(context: Context) = "${context.packageName}.files"
    fun dir(context: Context) = File(context.cacheDir, "archive")
}

data class ArchiveUiState(val uri: Uri, val fileName: String, val entries: List<ArchiveEntry>, val charset: Charset)

/**
 * ZIP archives: lists what is inside and opens a file from it in the matching viewer. A file is
 * unpacked into the app's cache, which is emptied when another archive is opened; the archive
 * itself is never changed.
 */
class ArchiveViewModel(app: Application) : AndroidViewModel(app) {
    private val recents = RecentFiles(app)

    var state by mutableStateOf<ArchiveUiState?>(null)
        private set
    var loading by mutableStateOf(false)
        private set
    var extracting by mutableStateOf(false)
        private set
    var error by mutableStateOf<String?>(null)
        private set

    fun open(uri: Uri) {
        val context = getApplication<Application>()
        val resolver = context.contentResolver
        loading = true
        viewModelScope.launch {
            val result = withContext(Dispatchers.IO) {
                runCatching {
                    ArchiveFiles.dir(context).deleteRecursively()
                    val info = FileInfo.of(resolver, uri)
                    val (entries, cs) = ArchiveReader.list { resolver.openInputStream(uri) ?: error("no data") }
                    recents.remember(uri, info.name, DocumentKind.ARCHIVE, info.size)
                    ArchiveUiState(uri, info.name, entries, cs)
                }
            }
            loading = false
            result.fold(onSuccess = { state = it }, onFailure = { error = it.message ?: it.javaClass.simpleName })
        }
    }

    /** Unpacks [entry] and hands its content:// address to [onReady]. */
    fun openEntry(entry: ArchiveEntry, onReady: (Uri) -> Unit) {
        val s = state ?: return
        val context = getApplication<Application>()
        extracting = true
        viewModelScope.launch {
            val result = withContext(Dispatchers.IO) {
                runCatching {
                    val dir = File(ArchiveFiles.dir(context), entry.index.toString())
                    val file = File(dir, ArchiveReader.safeName(entry.name)).takeIf { it.exists() }
                        ?: ArchiveReader.extract({ context.contentResolver.openInputStream(s.uri) ?: error("no data") }, s.charset, entry.index, dir)
                    FileProvider.getUriForFile(context, ArchiveFiles.authority(context), file)
                }
            }
            extracting = false
            result.fold(onSuccess = onReady, onFailure = { error = it.message ?: it.javaClass.simpleName })
        }
    }

    fun close() {
        state = null
    }

    fun consumeError() {
        error = null
    }
}
