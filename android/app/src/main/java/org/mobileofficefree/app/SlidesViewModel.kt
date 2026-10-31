package org.mobileofficefree.app

import android.app.Application
import android.graphics.Bitmap
import android.net.Uri
import androidx.core.graphics.createBitmap
import java.nio.ByteBuffer
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.viewModelScope
import java.util.Locale
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.mobileofficefree.engine.SlideText
import org.mobileofficefree.engine.SlidesDocument

/** Presentations (PPTX, PPSX, POTX) drawn by the DeckCraft renderer, one slide per page. */
class SlidesViewModel(app: Application) : PagedViewModel(app, DocumentKind.SLIDES) {
    private var doc: SlidesDocument? = null
    private var source: Uri? = null
    private var fileName = ""

    /** Editing: a tap on a slide opens the text under it; the bar adds, copies and deletes slides. */
    var editing by mutableStateOf(false)
    var dirty by mutableStateOf(false)
        private set
    /** The text being edited: its slide and shape. */
    var editingText by mutableStateOf<Pair<Int, SlideText>?>(null)
        private set

    override val hasUnsavedChanges: Boolean get() = dirty

    /** Saving writes the opened file again for the formats DeckCraft writes. */
    val savesInPlace: Boolean get() = fileName.substringAfterLast('.', "").lowercase(Locale.ROOT) in setOf("pptx", "ppsx", "potx")

    /** The text shape at (`x`, `y`) points on [slide], front-most first, offered for editing. */
    fun tapText(slide: Int, x: Float, y: Float) {
        val d = doc ?: return
        viewModelScope.launch {
            val hit = withContext(engine) {
                runCatching { d.textShapes(slide.toUInt()) }.getOrDefault(emptyList())
                    .lastOrNull { x >= it.x && x <= it.x + it.width && y >= it.y && y <= it.y + it.height }
            }
            editingText = hit?.let { slide to it }
            if (hit == null) tell(getApplication<Application>().getString(R.string.slide_no_text_here))
        }
    }

    fun cancelText() {
        editingText = null
    }

    fun setText(text: String) {
        val (slide, shape) = editingText ?: return
        editingText = null
        change {
            it.setText(slide.toUInt(), shape.id, text)
            null
        }
    }

    /** A slide to scroll to after an edit (the new or moved one), consumed by the screen. */
    var goTo by mutableStateOf<Int?>(null)

    fun newSlide(after: Int) = change { it.newSlide(after.toUInt()).toInt() }

    fun duplicateSlide(slide: Int) = change { it.duplicateSlide(slide.toUInt()).toInt() }

    fun deleteSlide(slide: Int) = change {
        it.deleteSlide(slide.toUInt())
        (slide - 1).coerceAtLeast(0)
    }

    fun moveSlide(from: Int, to: Int) = change {
        it.moveSlide(from.toUInt(), to.toUInt())
        to
    }

    /** Runs [op] on the engine thread (it may name a slide to show) and shows the slides as they are now. */
    private fun change(op: (SlidesDocument) -> Int?) {
        val d = doc ?: return
        val app = getApplication<Application>()
        viewModelScope.launch {
            val result = withContext(engine) {
                runCatching {
                    val show = op(d)
                    val size = d.slideSize()
                    List(d.slideCount().toInt()) { size.width to size.height } to show
                }
            }
            result.fold(
                onSuccess = { (pages, show) ->
                    dirty = true
                    pagesEdited(pages)
                    if (show != null) goTo = show
                },
                onFailure = { tell(app.getString(R.string.pdf_edit_failed, it.message ?: "")) },
            )
        }
    }

    fun saveInPlace(then: () -> Unit = {}) {
        source?.let { save(it, fileName, then) }
    }

    /** Writes the presentation into [dest] named [name] (PPTX, PPSX or POTX by its extension). */
    fun save(dest: Uri, name: String, then: () -> Unit = {}) {
        val d = doc ?: return
        val app = getApplication<Application>()
        viewModelScope.launch {
            val result = withContext(engine) {
                runCatching {
                    val bytes = d.save(name)
                    app.contentResolver.openOutputStream(dest, "wt")?.use { it.write(bytes) } ?: error("cannot write")
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
        val next = SlidesDocument.open(bytes)
        doc = next
        source = uri
        fileName = name
        val size = next.slideSize()
        return List(next.slideCount().toInt()) { size.width to size.height }
    }

    override fun renderPage(index: Int, widthPx: Int, night: Boolean): Bitmap? {
        val d = doc ?: return null
        val raster = d.renderSlide(index.toUInt(), widthPx / d.slideSize().width)
        // The renderer's premultiplied RGBA is exactly ARGB_8888's memory layout.
        return createBitmap(raster.width.toInt(), raster.height.toInt()).apply {
            copyPixelsFromBuffer(ByteBuffer.wrap(raster.pixels))
        }
    }

    /** The speaker notes of [slide] (empty when it has none). Runs on the engine thread. */
    suspend fun notes(slide: Int): String =
        withContext(engine) { runCatching { doc?.notes(slide.toUInt()) }.getOrNull().orEmpty() }

    override val exportFormats = listOf(PDF_FORMAT)

    override fun export(ext: String, name: String): ByteArray = doc?.exportPdf() ?: error("no document")

    override fun release() {
        doc?.close()
        doc = null
        source = null
        viewModelScope.launch {
            editing = false
            dirty = false
            editingText = null
        }
    }
}
