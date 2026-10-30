package org.mobileofficefree.app

import android.app.Application
import android.net.Uri
import android.os.SystemClock
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.text.input.TextFieldValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.mobileofficefree.engine.TextPage
import org.mobileofficefree.engine.WordDocument

/** One line on screen: a whole line of the file, or a slice of a very long one. */
data class TextLine(val number: Int, val text: String, val continued: Boolean)

data class TextUiState(
    val uri: Uri,
    val fileName: String,
    val lines: List<TextLine>,
    /** The same lines in [searchFold] form, for searching. */
    val folded: List<String>,
    val lineCount: Int,
    val encoding: TextEncoding,
    /** Only the first [TextViewModel.MAX_BYTES] were read. */
    val truncated: Boolean,
    /** The whole text with "\n" line ends, as the editor shows it. */
    val text: String,
    /** The file had Windows line ends ("\r\n"); saving puts them back. */
    val crlf: Boolean,
    /** The text with garbled Turkish letters put right, when it looks garbled (see [repairTurkish]). */
    val repaired: String? = null,
) {
    /** Small enough for the on-screen editor, and read whole. */
    val editable: Boolean get() = !truncated && text.length <= TextViewModel.MAX_EDIT_CHARS
}

/** What to do once the person has answered "save the changes?". */
enum class PendingLeave { StopEditing, Close }

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
                    val (raw, encoding) = TextEncoding.detect(if (truncated) bytes.copyOf(cutBeforeLastLetter(bytes, MAX_BYTES)) else bytes)
                    recents.remember(uri, info.name, DocumentKind.TEXT, info.size)
                    stateFor(uri, info.name, raw.replace("\r\n", "\n"), encoding, truncated, crlf = "\r\n" in raw)
                }
            }
            loading = false
            query = ""
            matches = emptyList()
            result.fold(onSuccess = { state = it }, onFailure = { error = it.message ?: it.javaClass.simpleName })
        }
    }

    private fun stateFor(uri: Uri, name: String, text: String, encoding: TextEncoding, truncated: Boolean, crlf: Boolean): TextUiState {
        val lines = splitLines(text)
        val repaired = if (!truncated && text.length <= MAX_EDIT_CHARS) repairTurkish(text) else null
        return TextUiState(uri, name, lines, lines.map { it.text.searchFold() }, lines.lastOrNull()?.number ?: 0, encoding, truncated, text, crlf, repaired)
    }

    // --- Editing ------------------------------------------------------------------------------

    var editing by mutableStateOf(false)
        private set
    var draft by mutableStateOf(TextFieldValue(""))
        private set
    var pendingLeave by mutableStateOf<PendingLeave?>(null)
        private set
    var notice by mutableStateOf<String?>(null)
        private set
    var busy by mutableStateOf(false)
        private set

    /** Changes not yet saved. */
    val dirty: Boolean get() = editing && draft.text != state?.text

    fun startEditing() {
        val s = state ?: return
        if (!s.editable) return
        draft = TextFieldValue(s.text)
        editing = true
    }

    /** Opens the editor on the repaired text; nothing is written until the person saves. */
    fun applyRepair() {
        val s = state ?: return
        val fixed = s.repaired ?: return
        draft = TextFieldValue(fixed)
        editing = true
        notice = getApplication<Application>().getString(R.string.text_repaired)
    }

    fun updateDraft(value: TextFieldValue) {
        draft = value
    }

    /** "Done": leaves the editor, asking first when there are unsaved changes. */
    fun finishEditing() {
        if (dirty) pendingLeave = PendingLeave.StopEditing else editing = false
    }

    /** Back or the arrow: closes the file, asking first when there are unsaved changes. */
    fun requestClose() {
        if (dirty) pendingLeave = PendingLeave.Close else close()
    }

    /** The answer to "save the changes?": save, discard, or (null) stay. */
    fun answerLeave(save: Boolean?) {
        val leave = pendingLeave ?: return
        pendingLeave = null
        when (save) {
            null -> Unit
            true -> save(then = leave)
            false -> {
                editing = false
                if (leave == PendingLeave.Close) close()
            }
        }
    }

    /**
     * Writes the draft over the file it came from, in its own encoding and line ends ("wt":
     * the whole file is replaced). Files unpacked from an archive live in the cache only, so
     * they are not saved in place.
     */
    fun save(then: PendingLeave? = null) {
        val s = state ?: return
        val app = getApplication<Application>()
        if (s.uri.authority == ArchiveFiles.authority(app)) {
            notice = app.getString(R.string.text_save_in_archive)
            return
        }
        val text = draft.text
        busy = true
        viewModelScope.launch {
            val result = withContext(Dispatchers.IO) {
                runCatching {
                    val bytes = s.encoding.encode(withLineEnds(text, s.crlf))
                    app.contentResolver.openOutputStream(s.uri, "wt")?.use { it.write(bytes) } ?: error("cannot write")
                    stateFor(s.uri, s.fileName, text, s.encoding, truncated = false, crlf = s.crlf)
                }
            }
            busy = false
            result.fold(
                onSuccess = {
                    state = it
                    notice = app.getString(R.string.saved)
                    when (then) {
                        PendingLeave.Close -> close()
                        PendingLeave.StopEditing -> editing = false
                        null -> Unit
                    }
                },
                onFailure = { notice = saveProblem(it) },
            )
        }
    }

    /** Writes the current text (the draft while editing) into the new file [uri] in [encoding]. */
    fun saveAs(uri: Uri, encoding: TextEncoding) {
        val s = state ?: return
        val app = getApplication<Application>()
        val text = if (editing) draft.text else s.text
        busy = true
        viewModelScope.launch {
            val result = withContext(Dispatchers.IO) {
                runCatching {
                    val bytes = encoding.encode(withLineEnds(text, s.crlf))
                    app.contentResolver.openOutputStream(uri, "wt")?.use { it.write(bytes) } ?: error("cannot write")
                }
            }
            busy = false
            notice = result.fold(onSuccess = { app.getString(R.string.saved) }, onFailure = ::saveProblem)
        }
    }

    /**
     * Converts the current text to [format] (PDF, DOCX, HTML…) with the WordCraft engine, which
     * reads each line as a paragraph laid out on [setup]'s pages, and writes it into the new file [uri].
     */
    fun exportTo(uri: Uri, format: ExportFormat, setup: TextPageSetup) {
        val s = state ?: return
        val app = getApplication<Application>()
        val text = if (editing) draft.text else s.text
        busy = true
        viewModelScope.launch {
            val started = SystemClock.elapsedRealtime()
            val result = withContext(converter) {
                runCatching {
                    val page = TextPage(setup.paper.widthPt, setup.paper.heightPt, setup.landscape, setup.marginPt, setup.font.family, setup.sizePt)
                    val bytes = WordDocument.fromText(text.toByteArray(), page).use { it.export(convertedName(s.fileName, format.ext)) }
                    app.contentResolver.openOutputStream(uri, "wt")?.use { it.write(bytes) } ?: error("cannot write")
                }
            }
            busy = false
            if (result.isSuccess) Diagnostics.record("TEXT → ${format.ext}", started)
            notice = result.fold(onSuccess = { app.getString(R.string.saved) }, onFailure = ::saveProblem)
        }
    }

    private fun saveProblem(e: Throwable): String {
        val app = getApplication<Application>()
        return when (e) {
            is UnencodableText -> app.getString(R.string.text_unencodable, e.char?.toString() ?: "?", e.encoding)
            else -> app.getString(R.string.save_failed, e.message ?: e.javaClass.simpleName)
        }
    }

    fun consumeNotice() {
        notice = null
    }

    fun close() {
        editing = false
        pendingLeave = null
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

    /** One engine document at a time for conversions (engine calls stay on one thread). */
    @OptIn(ExperimentalCoroutinesApi::class)
    private val converter = Dispatchers.IO.limitedParallelism(1)

    companion object {
        /** Bigger files show their beginning; a phone screen cannot page through more anyway. */
        const val MAX_BYTES = 16 * 1024 * 1024
        /** The on-screen editor stays responsive up to about this many characters. */
        const val MAX_EDIT_CHARS = 300_000
        private const val MAX_MATCHES = 10_000
    }
}

/** Very long lines (minified JSON) are cut into slices of this many characters, so each fits a list row. */
internal const val LINE_SLICE = 2_000

/** Decoded text and the encoding it was in (see [TextEncoding.detect]); the encoding's label for display. */
internal fun decodeText(bytes: ByteArray): Pair<String, String> = TextEncoding.detect(bytes).let { (t, e) -> t to e.label }

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
