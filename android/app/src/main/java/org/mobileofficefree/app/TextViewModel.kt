package org.mobileofficefree.app

import android.app.Application
import android.net.Uri
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import java.nio.ByteBuffer
import java.nio.charset.CharacterCodingException
import java.nio.charset.Charset
import java.nio.charset.CodingErrorAction
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** One line on screen: a whole line of the file, or a slice of a very long one. */
data class TextLine(val number: Int, val text: String, val continued: Boolean)

data class TextUiState(
    val uri: Uri,
    val fileName: String,
    val lines: List<TextLine>,
    /** The same lines in [searchFold] form, for searching. */
    val folded: List<String>,
    val lineCount: Int,
    val encoding: String,
    /** Only the first [TextViewModel.MAX_BYTES] were read. */
    val truncated: Boolean,
)

/** A search hit: display line and the range inside it. */
data class TextMatch(val line: Int, val start: Int, val end: Int)

/** Plain text, logs, JSON, XML, subtitles: shown as lines, with Turkish-aware encoding detection and search. */
class TextViewModel(app: Application) : AndroidViewModel(app) {
    private val recents = RecentFiles(app)

    var state by mutableStateOf<TextUiState?>(null)
        private set
    var loading by mutableStateOf(false)
        private set
    var error by mutableStateOf<String?>(null)
        private set

    var query by mutableStateOf("")
        private set
    var matches by mutableStateOf<List<TextMatch>>(emptyList())
        private set
    var currentMatch by mutableIntStateOf(0)
        private set
    private var searchJob: Job? = null

    fun open(uri: Uri) {
        val resolver = getApplication<Application>().contentResolver
        loading = true
        viewModelScope.launch {
            val result = withContext(Dispatchers.IO) {
                runCatching {
                    val info = FileInfo.of(resolver, uri)
                    val stream = resolver.openInputStream(uri) ?: error("no data")
                    val bytes = stream.use { it.readNBytesCompat(MAX_BYTES + 1) }
                    val truncated = bytes.size > MAX_BYTES
                    val (text, encoding) = decodeText(if (truncated) bytes.copyOf(cutBeforeLastLetter(bytes, MAX_BYTES)) else bytes)
                    val lines = splitLines(text)
                    recents.remember(uri, info.name, DocumentKind.TEXT, info.size)
                    TextUiState(uri, info.name, lines, lines.map { it.text.searchFold() }, lines.lastOrNull()?.number ?: 0, encoding, truncated)
                }
            }
            loading = false
            query = ""
            matches = emptyList()
            result.fold(onSuccess = { state = it }, onFailure = { error = it.message ?: it.javaClass.simpleName })
        }
    }

    fun close() {
        state = null
        query = ""
        matches = emptyList()
    }

    fun consumeError() {
        error = null
    }

    fun search(text: String) {
        query = text
        searchJob?.cancel()
        val s = state ?: return
        val needle = text.searchFold()
        if (needle.isEmpty()) {
            matches = emptyList()
            return
        }
        searchJob = viewModelScope.launch {
            matches = withContext(Dispatchers.Default) {
                val out = ArrayList<TextMatch>()
                for ((i, line) in s.folded.withIndex()) {
                    var at = line.indexOf(needle)
                    while (at >= 0 && out.size < MAX_MATCHES) {
                        out += TextMatch(i, at, at + needle.length)
                        at = line.indexOf(needle, at + needle.length)
                    }
                    if (out.size >= MAX_MATCHES) break
                }
                out
            }
            currentMatch = 0
        }
    }

    fun nextMatch(step: Int) {
        if (matches.isEmpty()) return
        currentMatch = Math.floorMod(currentMatch + step, matches.size)
    }

    companion object {
        /** Bigger files show their beginning; a phone screen cannot page through more anyway. */
        const val MAX_BYTES = 16 * 1024 * 1024
        private const val MAX_MATCHES = 10_000
    }
}

/** Very long lines (minified JSON) are cut into slices of this many characters, so each fits a list row. */
internal const val LINE_SLICE = 2_000

/** BOMs first; then strict UTF-8; otherwise Turkish Windows-1254, which most Turkish non-UTF-8 text is. */
internal fun decodeText(bytes: ByteArray): Pair<String, String> {
    fun starts(vararg b: Int) = bytes.size >= b.size && b.indices.all { (bytes[it].toInt() and 0xFF) == b[it] }
    return when {
        starts(0xEF, 0xBB, 0xBF) -> String(bytes, 3, bytes.size - 3, Charsets.UTF_8) to "UTF-8"
        starts(0xFF, 0xFE) -> String(bytes, 2, bytes.size - 2, Charsets.UTF_16LE) to "UTF-16"
        starts(0xFE, 0xFF) -> String(bytes, 2, bytes.size - 2, Charsets.UTF_16BE) to "UTF-16"
        else -> try {
            val strict = Charsets.UTF_8.newDecoder()
                .onMalformedInput(CodingErrorAction.REPORT)
                .onUnmappableCharacter(CodingErrorAction.REPORT)
            strict.decode(ByteBuffer.wrap(bytes)).toString() to "UTF-8"
        } catch (_: CharacterCodingException) {
            String(bytes, Charset.forName("windows-1254")) to "Windows-1254"
        }
    }
}

internal fun splitLines(text: String): List<TextLine> {
    val out = ArrayList<TextLine>()
    var number = 0
    for (raw in text.split('\n')) {
        number++
        val line = raw.removeSuffix("\r")
        if (line.length <= LINE_SLICE) {
            out += TextLine(number, line, false)
        } else {
            for (start in line.indices step LINE_SLICE) {
                out += TextLine(number, line.substring(start, minOf(start + LINE_SLICE, line.length)), start > 0)
            }
        }
    }
    // A final newline does not start another line.
    if (out.size > 1 && out.last().text.isEmpty() && !out.last().continued) out.removeAt(out.lastIndex)
    return out
}

/**
 * Where to cut [bytes] at or before [end] so no UTF-8 letter is split in two (a split one would
 * make the whole text look like it is not UTF-8). Drops the last non-ASCII letter, which is
 * harmless at the cut of a file too large to show whole.
 */
internal fun cutBeforeLastLetter(bytes: ByteArray, end: Int): Int {
    var i = end
    while (i > 0 && end - i < 3 && (bytes[i - 1].toInt() and 0xC0) == 0x80) i--
    if (i > 0 && (bytes[i - 1].toInt() and 0xC0) == 0xC0) i--
    return i
}

/** Up to [limit] bytes of the stream (InputStream.readNBytes needs API 33). */
private fun java.io.InputStream.readNBytesCompat(limit: Int): ByteArray {
    val out = java.io.ByteArrayOutputStream()
    val buf = ByteArray(64 * 1024)
    while (out.size() < limit) {
        val n = read(buf, 0, minOf(buf.size, limit - out.size()))
        if (n < 0) break
        out.write(buf, 0, n)
    }
    return out.toByteArray()
}
