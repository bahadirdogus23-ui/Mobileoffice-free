package org.mobileofficefree.app

import android.app.Application
import android.graphics.Bitmap
import android.net.Uri
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.core.graphics.createBitmap
import androidx.lifecycle.viewModelScope
import java.nio.ByteBuffer
import java.util.Locale
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject
import org.mobileofficefree.engine.SearchHit
import org.mobileofficefree.engine.WordCaret
import org.mobileofficefree.engine.WordDocument

/** The formatting where the caret is, for the editing tool bar. */
data class WordFormat(
    val bold: Boolean = false,
    val italic: Boolean = false,
    val underline: Boolean = false,
    /** Null when the selection mixes sizes or fonts. */
    val size: Float? = null,
    val font: String? = null,
)

/**
 * Word-processing documents (DOCX, DOC, ODT, RTF, …) drawn and edited by the WordCraft engine.
 * Every edit is an engine command (`text.insert`, `format.bold`, …) run on the engine thread.
 */
class WordViewModel(app: Application) : PagedViewModel(app, DocumentKind.WORD) {
    private var doc: WordDocument? = null
    private var source: Uri? = null
    private var fileName: String = ""

    var editing by mutableStateOf(false)
        private set
    var caret by mutableStateOf<WordCaret?>(null)
        private set
    var selection by mutableStateOf<List<SearchHit>>(emptyList())
        private set
    var format by mutableStateOf(WordFormat())
        private set
    var dirty by mutableStateOf(false)
        private set
    /** The selected text (short), offered as what to find when replacing. */
    var selectedTextHint by mutableStateOf("")
        private set

    override val hasUnsavedChanges: Boolean get() = dirty

    /** Saving writes the opened file again only in formats the engine writes faithfully. */
    val savesInPlace: Boolean get() = fileName.substringAfterLast('.', "").lowercase(Locale.ROOT) in IN_PLACE

    fun startEditing() {
        editing = true
        if (caret == null) command("caret.docStart", changes = false)
    }

    fun stopEditing() {
        editing = false
    }

    /** Caret to (`x`, `y`) points on [page], or the selection extended there. */
    fun tap(page: Int, x: Float, y: Float, extend: Boolean = false) = perform(changes = false) { it.tap(page.toUInt(), x, y, extend) }

    /** The word at (`x`, `y`) selected. */
    fun selectWord(page: Int, x: Float, y: Float) = perform(changes = false) {
        it.tap(page.toUInt(), x, y, false)
        it.execute("select.word", "")
    }

    /** The selection extended to (`x`, `y`) on [page] (dragging after a long press). */
    fun extendTo(page: Int, x: Float, y: Float) = tap(page, x, y, extend = true)

    /** The selected text, handed to [onText] on the main thread (for the clipboard). */
    fun copy(onText: (String) -> Unit) {
        val d = doc ?: return
        viewModelScope.launch {
            val text = withContext(engine) { runCatching { d.selectedText() }.getOrDefault("") }
            if (text.isNotEmpty()) onText(text)
        }
    }

    /** Copy, then delete the selection. */
    fun cut(onText: (String) -> Unit) {
        copy { text ->
            onText(text)
            command("text.delete")
        }
    }

    /** Clipboard text at the caret; [html] keeps its formatting when the clipboard has it. */
    fun paste(text: String, html: String?) {
        val params = JSONObject().put("text", text)
        if (html != null) params.put("html", html)
        command("edit.paste", params.toString())
    }

    /** A picture from [uri] at the caret, [WIDTH_PT] wide at most (it keeps its proportions). */
    fun insertPicture(uri: Uri) {
        val app = getApplication<Application>()
        perform(changes = true) { d ->
            val bytes = app.contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: error("no data")
            val data = android.util.Base64.encodeToString(bytes, android.util.Base64.NO_WRAP)
            d.execute("insert.picture", JSONObject().put("data", data).put("width", PICTURE_WIDTH_PT).toString())
        }
    }

    fun insertTable(rows: Int, cols: Int) = command("insert.table", JSONObject().put("rows", rows).put("cols", cols).toString())

    /** Replaces every [find] with [with]; tells how it went. */
    fun replaceAll(find: String, with: String) = perform(changes = true) { d ->
        val before = d.find(find).map { it.index }.distinct().size
        d.execute("edit.replaceAll", JSONObject().put("text", find).put("with", with).toString())
        val app = getApplication<Application>()
        tellLater(app.resources.getQuantityString(R.plurals.replaced, before, before))
    }

    fun setSize(points: Float) = command("format.size", JSONObject().put("size", points.toDouble()).toString())

    fun setFont(name: String) = command("format.font", JSONObject().put("name", name).toString())

    /** Typed text: lines become paragraphs. */
    fun type(text: String) = perform(changes = true) { d ->
        text.replace("\r\n", "\n").split('\n').forEachIndexed { i, line ->
            if (i > 0) d.execute("text.newParagraph", "")
            if (line.isNotEmpty()) d.execute("text.insert", JSONObject().put("text", line).toString())
        }
    }

    /** Runs engine command [id]; [changes] says whether it can change the text or its look. */
    fun command(id: String, params: String = "", changes: Boolean = true) = perform(changes) { it.execute(id, params) }

    private fun perform(changes: Boolean, op: (WordDocument) -> Unit) {
        val d = doc ?: return
        viewModelScope.launch {
            val result = withContext(engine) {
                runCatching {
                    op(d)
                    val sizes = if (changes) (0u until d.pageCount()).map { d.pageSize(it).let { s -> s.width to s.height } } else null
                    val f = runCatching { JSONObject(d.execute("format.state", "")) }.getOrNull()
                    Edited(sizes, d.caret(), d.selection(), f?.let { formatOf(it) } ?: WordFormat(), d.isDirty(), d.selectedText().take(200))
                }
            }
            result.onSuccess { e ->
                caret = e.caret
                selection = e.selection
                format = e.format
                dirty = e.dirty
                selectedTextHint = e.selected
                e.sizes?.let(::pagesEdited)
            }
        }
    }

    /** A message from the engine thread, shown on the main thread. */
    private fun tellLater(text: String) {
        viewModelScope.launch { tell(text) }
    }

    private class Edited(val sizes: List<Pair<Float, Float>>?, val caret: WordCaret?, val selection: List<SearchHit>, val format: WordFormat, val dirty: Boolean, val selected: String)

    private fun formatOf(o: JSONObject) = WordFormat(
        bold = o.optBoolean("bold"),
        italic = o.optBoolean("italic"),
        underline = o.opt("underline").let { it == true || (it is String && it != "none" && it.isNotEmpty()) },
        size = (o.opt("size") as? Number)?.toFloat(),
        font = o.opt("font") as? String,
    )

    /** Writes the document over the opened file, in its own format. */
    fun saveInPlace(then: () -> Unit = {}) {
        val uri = source ?: return
        save(uri, fileName, then)
    }

    /** Writes the document into the new file [dest] named [name] (its extension picks the format). */
    fun save(dest: Uri, name: String, then: () -> Unit = {}) {
        val d = doc ?: return
        val app = getApplication<Application>()
        viewModelScope.launch {
            val result = withContext(engine) {
                runCatching {
                    val bytes = d.export(name)
                    app.contentResolver.openOutputStream(dest, "wt")?.use { it.write(bytes) } ?: error("cannot write")
                    d.markSaved()
                }
            }
            if (result.isSuccess) {
                dirty = false
                then()
            }
            tell(result.fold(onSuccess = { app.getString(R.string.saved) }, onFailure = { app.getString(R.string.save_failed, it.message ?: "") }))
        }
    }

    override fun load(uri: Uri, name: String): List<Pair<Float, Float>> {
        val bytes = getApplication<Application>().contentResolver.openInputStream(uri)?.use { it.readBytes() }
            ?: error("no data")
        val next = WordDocument.open(name, bytes, null)
        doc = next
        source = uri
        fileName = name
        return (0u until next.pageCount()).map { next.pageSize(it).let { s -> s.width to s.height } }
    }

    /** WordCraft draws real dark pages: text and shapes flip lightness, pictures keep their colours. */
    override val drawsNight = true

    override fun renderPage(index: Int, widthPx: Int, night: Boolean): Bitmap? {
        val d = doc ?: return null
        val size = d.pageSize(index.toUInt())
        val raster = d.renderPage(index.toUInt(), widthPx / size.width, night)
        // The engine's premultiplied RGBA is exactly ARGB_8888's memory layout.
        return createBitmap(raster.width.toInt(), raster.height.toInt()).apply {
            copyPixelsFromBuffer(ByteBuffer.wrap(raster.pixels))
        }
    }

    override val searchable = true

    override val canReadAloud = true

    override fun openedNotice(): String? =
        if (doc?.isSigned() == true) getApplication<Application>().getString(R.string.udf_signed) else null

    override val exportFormats = WORD_FORMATS

    override fun export(ext: String, name: String): ByteArray = doc?.export(name) ?: error("no document")

    override fun pageTexts(): List<String> = doc?.pageTexts().orEmpty()

    override fun findText(query: String): List<FoundRect> =
        doc?.find(query).orEmpty().map { FoundRect(it.index.toInt(), it.page.toInt(), it.x, it.y, it.x + it.width, it.y + it.height) }

    override fun release() {
        doc?.close()
        doc = null
        source = null
        viewModelScope.launch {
            editing = false
            caret = null
            selection = emptyList()
            dirty = false
        }
    }
}

/** Pictures go in this wide (points) at most: a little under an A4 page's text width. */
private const val PICTURE_WIDTH_PT = 400.0

/** Formats the engine writes back without losing what it read. */
private val IN_PLACE = setOf("docx", "docm", "odt", "rtf", "md", "txt", "html", "htm")
