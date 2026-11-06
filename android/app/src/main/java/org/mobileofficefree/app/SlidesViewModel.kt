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
import org.mobileofficefree.engine.SlideObject
import org.mobileofficefree.engine.SlideObjectKind
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
                    .filter { x >= it.x && x <= it.x + it.width && y >= it.y && y <= it.y + it.height }
                    // The smallest box under the finger: a big text box behind (or over) a
                    // smaller one must not swallow the tap.
                    .minByOrNull { it.width * it.height }
            }
            editingText = hit?.let { slide to it }
            if (hit == null) tell(getApplication<Application>().getString(R.string.slide_no_text_here))
        }
    }

    /** The object picked on a slide (to move, resize, delete or retype), and where it was tapped. */
    var picked by mutableStateOf<Pair<Int, SlideObject>?>(null)
        private set
    private var pickedAt = 0f to 0f

    /** Picks the object at (`x`, `y`) points on [slide]: the smallest box under the finger. */
    fun pickAt(slide: Int, x: Float, y: Float) {
        val d = doc ?: return
        viewModelScope.launch {
            val hit = withContext(engine) {
                runCatching { d.objects(slide.toUInt()) }.getOrDefault(emptyList())
                    .filter { x >= it.x && x <= it.x + it.width && y >= it.y && y <= it.y + it.height }
                    .minByOrNull { it.width * it.height }
            }
            pickedAt = x to y
            picked = hit?.let { slide to it }
            pickedLook = null
            pickedBold = hit != null && withContext(engine) { runCatching { d.textIsBold(slide.toUInt(), hit.id) }.getOrDefault(false) }
            if (hit != null) refreshLook(slide, hit.id)
        }
    }

    /** The picked object on its own and the slide behind it, for dragging it (see [ObjectFrame]). */
    var pickedLook by mutableStateOf<ObjectLook?>(null)
        private set

    /** Draws the slide with and without object [id] and keeps the difference as [pickedLook]. */
    private fun refreshLook(slide: Int, id: UInt) {
        val d = doc ?: return
        viewModelScope.launch {
            val look = withContext(engine) {
                runCatching {
                    val scale = LOOK_WIDTH_PX / d.slideSize().width
                    val full = d.renderSlide(slide.toUInt(), scale).toBitmap()
                    val without = d.renderSlideWithout(slide.toUInt(), id, scale).toBitmap()
                    objectSnapshot(full, without).also { full.recycle() }
                }.getOrNull()
            }
            if (picked?.second?.id == id) pickedLook = look
        }
    }

    private fun org.mobileofficefree.engine.Raster.toBitmap(): Bitmap =
        createBitmap(width.toInt(), height.toInt()).apply { copyPixelsFromBuffer(ByteBuffer.wrap(pixels)) }

    fun dropPick() {
        picked = null
        pickedLook = null
    }

    /** Whether the picked object has text to retype. */
    val pickedHasText: Boolean
        get() = picked?.second?.kind.let { it == SlideObjectKind.TEXT || it == SlideObjectKind.TABLE }

    /** Opens the text of the picked object (for a table, the cell that was tapped). */
    fun editPickedText() {
        val (slide, o) = picked ?: return
        val (x, y) = pickedAt
        // Tapped before a move: aim at the same spot of the object where it is now.
        tapText(slide, x.coerceIn(o.x, o.x + o.width), y.coerceIn(o.y, o.y + o.height))
    }

    /**
     * Moves the picked object by (`dx`, `dy`) points, scales it by [scale] about its centre and
     * turns it [turn] degrees clockwise.
     */
    fun placePicked(dx: Double, dy: Double, scale: Double = 1.0, turn: Double = 0.0) {
        val (slide, o) = picked ?: return
        pickedAt = (pickedAt.first + dx.toFloat()) to (pickedAt.second + dy.toFloat())
        change {
            it.placeObject(slide.toUInt(), o.id, dx, dy, scale, turn)
            val now = it.objects(slide.toUInt()).firstOrNull { n -> n.id == o.id }
            viewModelScope.launch {
                picked = now?.let { n -> slide to n }
                if (now != null) refreshLook(slide, now.id)
            }
            null
        }
    }

    /** A new text box reading [text] in the middle of [slide], picked so it can be moved at once. */
    fun addTextBox(slide: Int, text: String) {
        change {
            val id = it.addTextBox(slide.toUInt(), text)
            pickLater(it, slide, id)
            slide
        }
    }

    /**
     * The picture at [uri] in the middle of [slide], picked. Photos over 2560 pixels are made
     * smaller, and pictures other than PNG and JPEG (HEIC, WebP…) are stored as JPEG.
     */
    fun addPicture(slide: Int, uri: Uri) {
        val app = getApplication<Application>()
        change {
            val bytes = app.contentResolver.openInputStream(uri)?.use { s -> s.readBytes() } ?: error("no data")
            val bounds = android.graphics.BitmapFactory.Options().apply { inJustDecodeBounds = true }
            android.graphics.BitmapFactory.decodeByteArray(bytes, 0, bytes.size, bounds)
            if (bounds.outWidth <= 0 || bounds.outHeight <= 0) error(app.getString(R.string.slide_picture_unreadable))
            val png = bytes.size > 4 && bytes[0] == 0x89.toByte() && bytes[1] == 'P'.code.toByte()
            val jpeg = bytes.size > 2 && bytes[0] == 0xFF.toByte() && bytes[1] == 0xD8.toByte()
            val big = maxOf(bounds.outWidth, bounds.outHeight) > MAX_PICTURE_PX
            val (data, w, h) = if ((png || jpeg) && !big) {
                Triple(bytes, bounds.outWidth, bounds.outHeight)
            } else {
                var sample = 1
                while (maxOf(bounds.outWidth, bounds.outHeight) / (sample * 2) >= MAX_PICTURE_PX) sample *= 2
                val decoded = android.graphics.BitmapFactory.decodeByteArray(bytes, 0, bytes.size, android.graphics.BitmapFactory.Options().apply { inSampleSize = sample })
                    ?: error(app.getString(R.string.slide_picture_unreadable))
                val k = minOf(1f, MAX_PICTURE_PX.toFloat() / maxOf(decoded.width, decoded.height))
                val scaled = if (k < 1f) android.graphics.Bitmap.createScaledBitmap(decoded, (decoded.width * k).toInt().coerceAtLeast(1), (decoded.height * k).toInt().coerceAtLeast(1), true) else decoded
                val out = java.io.ByteArrayOutputStream()
                // A PNG keeps its transparency; anything else becomes a JPEG.
                scaled.compress(if (png) android.graphics.Bitmap.CompressFormat.PNG else android.graphics.Bitmap.CompressFormat.JPEG, 90, out)
                Triple(out.toByteArray(), scaled.width, scaled.height)
            }
            val id = it.addPicture(slide.toUInt(), data, w.toUInt(), h.toUInt())
            pickLater(it, slide, id)
            slide
        }
    }

    /** Runs on the engine thread: picks object [id] of [slide] once the change shows. */
    private fun pickLater(d: SlidesDocument, slide: Int, id: UInt) {
        val o = d.objects(slide.toUInt()).firstOrNull { it.id == id } ?: return
        viewModelScope.launch {
            pickedAt = (o.x + o.width / 2) to (o.y + o.height / 2)
            picked = slide to o
            refreshLook(slide, o.id)
        }
    }

    /** Whether the picked object's text starts bold (for the bold button). */
    var pickedBold by mutableStateOf(false)
        private set

    /** A new shape (PowerPoint's name, e.g. "ellipse") in the middle of [slide], picked. */
    fun addShape(slide: Int, preset: String) {
        change {
            val id = it.addShape(slide.toUInt(), preset, SHAPE_BLUE)
            pickLater(it, slide, id)
            slide
        }
    }

    /** A copy of the picked object, picked in its place. */
    fun duplicatePicked() {
        val (slide, o) = picked ?: return
        change {
            val id = it.duplicateObject(slide.toUInt(), o.id)
            pickLater(it, slide, id)
            null
        }
    }

    /** The picked object brought in front of the others ([front]) or sent behind them. */
    fun arrangePicked(front: Boolean) = restyle { d, slide, id -> d.arrangeObject(slide, id, front) }

    /** Changes the look of the picked object's text or box. */
    fun stylePicked(style: org.mobileofficefree.engine.SlideStyle) = restyle { d, slide, id -> d.styleObject(slide, id, style) }

    private fun restyle(op: (SlidesDocument, UInt, UInt) -> Unit) {
        val (slide, o) = picked ?: return
        change {
            op(it, slide.toUInt(), o.id)
            val bold = it.textIsBold(slide.toUInt(), o.id)
            pickLater(it, slide, o.id)
            viewModelScope.launch { pickedBold = bold }
            null
        }
    }

    /**
     * The picked object given the box centred on (`cx`, `cy`), `w`×`h` points (pulled from a
     * side): text flows anew in it, a picture or shape stretches.
     */
    fun setPickedBox(cx: Double, cy: Double, w: Double, h: Double) {
        val (slide, o) = picked ?: return
        change {
            it.setObjectBox(slide.toUInt(), o.id, cx - w / 2, cy - h / 2, w, h)
            val now = it.objects(slide.toUInt()).firstOrNull { n -> n.id == o.id }
            viewModelScope.launch {
                picked = now?.let { n -> slide to n }
                if (now != null) refreshLook(slide, now.id)
            }
            null
        }
    }

    fun deletePicked() {
        val (slide, o) = picked ?: return
        picked = null
        change {
            it.deleteObject(slide.toUInt(), o.id)
            null
        }
    }

    fun cancelText() {
        editingText = null
    }

    fun setText(text: String) {
        val (slide, shape) = editingText ?: return
        editingText = null
        change {
            if (shape.row >= 0) it.setCellText(slide.toUInt(), shape.id, shape.row.toUInt(), shape.col.toUInt(), text) else it.setText(slide.toUInt(), shape.id, text)
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

    /** Whether a change can be taken back, or made again. */
    var canUndo by mutableStateOf(false)
        private set
    var canRedo by mutableStateOf(false)
        private set

    /** Takes back the last change (the picked object is let go: it may be gone). */
    fun undo() {
        picked = null
        pickedLook = null
        change { it.undo(); null }
    }

    fun redo() {
        picked = null
        pickedLook = null
        change { it.redo(); null }
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
                    Triple(List(d.slideCount().toInt()) { size.width to size.height }, show, d.canUndo() to d.canRedo())
                }
            }
            result.fold(
                onSuccess = { (pages, show, history) ->
                    canUndo = history.first
                    canRedo = history.second
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

    /** Makes [slide]'s speaker notes read [text]. */
    fun setNotes(slide: Int, text: String) = change {
        it.setNotes(slide.toUInt(), text)
        null
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
            picked = null
            pickedLook = null
            canUndo = false
            canRedo = false
        }
    }
}

/** Pictures added to slides are made at most this many pixels on their long side. */
private const val MAX_PICTURE_PX = 2560

/** New shapes' colour (Office blue), changed with Fill afterwards. */
private const val SHAPE_BLUE = 0x4472C4u

/** How wide (pixels) a slide is drawn to cut a picked object out of it. */
private const val LOOK_WIDTH_PX = 1080f
