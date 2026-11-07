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

/** [sevenZip]: the 7z archive copied into the cache (7z is read from a file), null for a ZIP. */
data class ArchiveUiState(val uri: Uri, val fileName: String, val entries: List<ArchiveEntry>, val charset: Charset, val sevenZip: File? = null)

/**
 * ZIP and 7z archives: lists what is inside and opens a file from it in the matching viewer. A file is
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
                    // The first bytes say which kind of archive it is (read the long way: Android 8 has no readNBytes).
                    val head = ByteArray(8)
                    resolver.openInputStream(uri)?.use { input ->
                        var got = 0
                        while (got < head.size) {
                            val n = input.read(head, got, head.size - got)
                            if (n < 0) break
                            got += n
                        }
                    } ?: error("no data")
                    val state = if (org.mobileofficefree.engine.isSevenZip(head)) {
                        // 7z: the engine reads it from a file (it seeks), so it is copied into the cache.
                        val copy = File(ArchiveFiles.dir(context), "archive.7z").apply { parentFile?.mkdirs() }
                        resolver.openInputStream(uri)?.use { input -> copy.outputStream().use { input.copyTo(it) } } ?: error("no data")
                        val entries = org.mobileofficefree.engine.sevenZipEntries(copy.path)
                            .filter { !it.isDir }
                            .take(ArchiveReader.MAX_ENTRIES)
                            .mapIndexed { i, e -> ArchiveEntry(i, e.path, e.size.toLong()) }
                        if (entries.isEmpty()) error("no files in archive")
                        ArchiveUiState(uri, info.name, entries, Charsets.UTF_8, copy)
                    } else {
                        val (entries, cs) = ArchiveReader.list { resolver.openInputStream(uri) ?: error("no data") }
                        ArchiveUiState(uri, info.name, entries, cs)
                    }
                    recents.remember(uri, info.name, DocumentKind.ARCHIVE, info.size)
                    state
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
                    val seven = s.sevenZip
                    val file = File(dir, ArchiveReader.safeName(entry.name)).takeIf { it.exists() }
                        ?: if (seven != null) {
                            val bytes = org.mobileofficefree.engine.sevenZipExtract(seven.path, entry.path, ArchiveReader.MAX_ENTRY_BYTES.toULong())
                            dir.mkdirs()
                            File(dir, ArchiveReader.safeName(entry.name)).apply { writeBytes(bytes) }
                        } else {
                            ArchiveReader.extract({ context.contentResolver.openInputStream(s.uri) ?: error("no data") }, s.charset, entry.index, dir)
                        }
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
