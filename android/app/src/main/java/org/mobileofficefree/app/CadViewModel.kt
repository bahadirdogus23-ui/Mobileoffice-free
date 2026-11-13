package org.mobileofficefree.app

import android.app.Application
import android.graphics.Bitmap
import android.net.Uri
import android.os.SystemClock
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.core.graphics.createBitmap
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import java.nio.ByteBuffer
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.mobileofficefree.engine.CadDocument
import org.mobileofficefree.engine.CadLayer

data class CadUiState(
    val uri: Uri,
    val fileName: String,
    val spaces: List<String>,
    val space: Int,
    val layers: List<CadLayer>,
    val dark: Boolean,
    val notes: List<String>,
)

/** Where the drawing is looked at: world centre and pixels per drawing unit. */
data class CadView(val centerX: Double, val centerY: Double, val scale: Double)

/** A drawn view and the view it was drawn for (gestures move it until the next one is ready). */
class CadFrame(val bitmap: Bitmap, val view: CadView)

/** DWG/DXF viewing through the CADCraft engine (view only, by design: plan §7.6). */
class CadViewModel(app: Application) : AndroidViewModel(app) {

    @OptIn(ExperimentalCoroutinesApi::class)
    private val engine: CoroutineDispatcher = Dispatchers.IO.limitedParallelism(1)

    private var doc: CadDocument? = null
    private val recents = RecentFiles(app)

    var state by mutableStateOf<CadUiState?>(null)
        private set
    var loading by mutableStateOf(false)
        private set
    var error by mutableStateOf<String?>(null)
        private set

    /** The view the person is looking at now; it follows the fingers. */
    var view by mutableStateOf<CadView?>(null)
        private set
    var frame by mutableStateOf<CadFrame?>(null)
        private set

    private var viewport = 0 to 0
    private var renderedFor: Pair<CadView, Pair<Int, Int>>? = null

    /** The render in flight; views asked for meanwhile are drawn once it ends (only the newest). */
    private var renderJob: Job? = null
    private var renderAgain = false

    /** The picture shown before the current one: the next render draws into it instead of a new one. */
    private var spare: Bitmap? = null

    /** Bumped when a document opens or closes: a render of the previous one is then dropped. */
    private var generation = 0

    fun open(uri: Uri) {
        val resolver = getApplication<Application>().contentResolver
        loading = true
        generation++
        val started = SystemClock.elapsedRealtime()
        viewModelScope.launch {
            val result = withContext(engine) {
                runCatching {
                    doc?.close()
                    doc = null
                    val info = FileInfo.of(resolver, uri)
                    // The file's bytes live only for this call: the engine keeps the drawing it
                    // reads, not the file (a large DWG is tens of MB on top of the drawing).
                    val next = CadDocument.open(info.typedName, readAll(resolver, uri, info.size))
                    Diagnostics.record("CAD ${info.typedName.substringAfterLast('.')} açma (${sizeLabel(info.size)})", started)
                    doc = next
                    recents.remember(uri, info.name, DocumentKind.CAD, info.size)
                    CadUiState(uri, info.name, next.spaceNames(), 0, next.layers(), dark = true, notes = next.notes())
                }
            }
            loading = false
            frame = null
            spare = null
            view = null
            result.fold(onSuccess = { state = it; fit() }, onFailure = { error = it.message ?: it.javaClass.simpleName })
        }
    }

    fun close() {
        state = null
        frame = null
        spare = null
        view = null
        generation++
        viewModelScope.launch(engine) {
            doc?.close()
            doc = null
        }
    }

    fun consumeError() {
        error = null
    }

    /** The screen area the drawing fills; the first one fits the whole drawing. */
    fun setViewport(width: Int, height: Int) {
        if (width <= 0 || height <= 0 || viewport == width to height) return
        viewport = width to height
        if (view == null) fit() else render()
    }

    /** Fits the whole of the current space on screen. */
    fun fit() {
        val s = state ?: return
        val (w, h) = viewport
        if (w <= 0) return
        viewModelScope.launch {
            val e = withContext(engine) { runCatching { doc?.extents(s.space.toUInt()) }.getOrNull() } ?: return@launch
            val ew = (e.maxX - e.minX).coerceAtLeast(1e-9)
            val eh = (e.maxY - e.minY).coerceAtLeast(1e-9)
            view = CadView((e.minX + e.maxX) / 2, (e.minY + e.maxY) / 2, minOf(w / ew, h / eh) * 0.95)
            render()
        }
    }

    /** A pinch/pan step: [zoom] about screen point ([cx], [cy]), then move by ([dx], [dy]) pixels. */
    fun transform(cx: Float, cy: Float, dx: Float, dy: Float, zoom: Float) {
        val v = view ?: return
        val (w, h) = viewport
        val s1 = (v.scale * zoom).coerceIn(1e-9, 1e9)
        // The world point under the fingers stays under them while zooming (screen y points down).
        val wx = v.centerX + (cx - w / 2.0) / v.scale
        val wy = v.centerY - (cy - h / 2.0) / v.scale
        val ncx = wx - (cx - w / 2.0) / s1 - dx / s1
        val ncy = wy + (cy - h / 2.0) / s1 + dy / s1
        view = CadView(ncx, ncy, s1)
    }

    /**
     * Draws the current view sharply (called when a gesture settles). One render runs at a time;
     * views asked for while it runs are not queued one by one: once it ends, only the newest is
     * drawn. The picture before the current one is reused as the canvas of the next.
     */
    fun render() {
        val s = state ?: return
        val v = view ?: return
        val (w, h) = viewport
        if (w <= 0 || renderedFor == (v to viewport)) return
        if (renderJob?.isActive == true) {
            renderAgain = true
            return
        }
        renderedFor = v to viewport
        val gen = generation
        val reuse = spare?.takeIf { !it.isRecycled && it.width == w && it.height == h }
        spare = null
        renderJob = viewModelScope.launch {
            try {
                val bmp = withContext(engine) {
                    runCatching {
                        val r = Measurements.time("CAD: görünüm çizimi") { doc?.render(s.space.toUInt(), v.centerX, v.centerY, v.scale, w.toUInt(), h.toUInt(), s.dark) }
                            ?: return@runCatching null
                        // tiny-skia's premultiplied RGBA is exactly ARGB_8888's memory layout.
                        val target = reuse?.takeIf { it.width == r.width.toInt() && it.height == r.height.toInt() }
                            ?: createBitmap(r.width.toInt(), r.height.toInt())
                        target.apply { copyPixelsFromBuffer(ByteBuffer.wrap(r.pixels)) }
                    }.getOrNull()
                }
                if (bmp != null && gen == generation) {
                    val shown = frame?.bitmap
                    frame = CadFrame(bmp, v)
                    if (shown !== bmp) spare = shown
                }
            } finally {
                renderJob = null
                if (renderAgain) {
                    renderAgain = false
                    render()
                }
            }
        }
    }

    fun selectSpace(index: Int) {
        val s = state ?: return
        state = s.copy(space = index)
        renderedFor = null
        fit()
    }

    /** Measuring: the points tapped so far (world units), at most two. */
    var measuring by mutableStateOf(false)
    var measure by mutableStateOf<List<Pair<Double, Double>>>(emptyList())
        private set

    /** The world point under screen point ([x], [y]) of the current view. */
    fun worldAt(x: Float, y: Float): Pair<Double, Double>? {
        val v = view ?: return null
        val (w, h) = viewport
        return (v.centerX + (x - w / 2.0) / v.scale) to (v.centerY - (y - h / 2.0) / v.scale)
    }

    /** Screen point of world point [p] in the current view. */
    fun screenOf(p: Pair<Double, Double>): Pair<Float, Float>? {
        val v = view ?: return null
        val (w, h) = viewport
        return ((p.first - v.centerX) * v.scale + w / 2.0).toFloat() to (-(p.second - v.centerY) * v.scale + h / 2.0).toFloat()
    }

    /** What each measured point snapped to ("Endpoint", …), or "" where it did not. */
    var measureSnaps by mutableStateOf<List<String>>(emptyList())
        private set

    /**
     * A tap while measuring: the first or second point, or a fresh start after two. The point
     * snaps to the nearest end, middle, centre or crossing within a fingertip of the tap.
     */
    fun measureTap(x: Float, y: Float) {
        val raw = worldAt(x, y) ?: return
        val s = state ?: return
        val v = view ?: return
        val fingertip = 24.0 * getApplication<Application>().resources.displayMetrics.density / v.scale
        viewModelScope.launch {
            val hit = withContext(engine) { runCatching { doc?.snap(s.space.toUInt(), raw.first, raw.second, fingertip) }.getOrNull() }
            val p = hit?.let { it.x to it.y } ?: raw
            val kind = hit?.kind.orEmpty()
            if (measure.size >= 2) {
                measure = listOf(p)
                measureSnaps = listOf(kind)
            } else {
                measure = measure + p
                measureSnaps = measureSnaps + kind
            }
        }
    }

    /** While a finger aims: the point a lift would measure (snapped, with what it snapped to, "" if free). */
    var aim by mutableStateOf<Pair<Pair<Double, Double>, String>?>(null)
        private set
    private var aimSeq = 0

    /** The finger aims at screen point ([x], [y]): the snap there is looked up and shown before it is chosen. */
    fun aimAt(x: Float, y: Float) {
        val raw = worldAt(x, y) ?: return
        val s = state ?: return
        val v = view ?: return
        val fingertip = 32.0 * getApplication<Application>().resources.displayMetrics.density / v.scale
        val seq = ++aimSeq
        viewModelScope.launch {
            val hit = withContext(engine) { runCatching { doc?.snap(s.space.toUInt(), raw.first, raw.second, fingertip) }.getOrNull() }
            // Only the newest answer counts (the finger has moved on otherwise).
            if (seq == aimSeq) aim = (hit?.let { it.x to it.y } ?: raw) to hit?.kind.orEmpty()
        }
    }

    /** The finger lifted: the aimed point is measured. */
    fun aimDone() {
        val (p, kind) = aim ?: return
        aim = null
        aimSeq++
        if (measure.size >= 2) {
            measure = listOf(p)
            measureSnaps = listOf(kind)
        } else {
            measure = measure + p
            measureSnaps = measureSnaps + kind
        }
    }

    fun aimCancel() {
        aim = null
        aimSeq++
    }

    fun stopMeasuring() {
        measuring = false
        measure = emptyList()
        measureSnaps = emptyList()
    }

    /**
     * Writes the current space, whole, as a one-page PDF into [dest]: landscape A4 or portrait,
     * whichever the drawing fits better, drawn at 200 dpi on white.
     */
    /** Whether the next PDF holds only the area on screen (true) or the whole drawing. */
    var pdfOnlyView = false

    fun exportPdf(dest: Uri) {
        val s = state ?: return
        val app = getApplication<Application>()
        // The area on screen, in drawing units, when only it goes in the PDF.
        val shown = view?.takeIf { pdfOnlyView }?.let { v ->
            val (vw, vh) = viewport
            doubleArrayOf(v.centerX, v.centerY, vw / v.scale, vh / v.scale)
        }
        viewModelScope.launch {
            val result = withContext(engine) {
                runCatching {
                    val d = doc ?: error("no document")
                    val e = d.extents(s.space.toUInt())
                    val cx = shown?.get(0) ?: ((e.minX + e.maxX) / 2)
                    val cy = shown?.get(1) ?: ((e.minY + e.maxY) / 2)
                    val ew = (shown?.get(2) ?: (e.maxX - e.minX)).coerceAtLeast(1e-9)
                    val eh = (shown?.get(3) ?: (e.maxY - e.minY)).coerceAtLeast(1e-9)
                    val (pw, ph) = if (ew >= eh) 842 to 595 else 595 to 842
                    val margin = 24
                    val dpi = 200.0 / 72.0
                    val w = ((pw - 2 * margin) * dpi).toInt()
                    val h = ((ph - 2 * margin) * dpi).toInt()
                    val scale = minOf(w / ew, h / eh)
                    val r = d.render(s.space.toUInt(), cx, cy, scale, w.toUInt(), h.toUInt(), false)
                    val bmp = createBitmap(r.width.toInt(), r.height.toInt()).apply { copyPixelsFromBuffer(ByteBuffer.wrap(r.pixels)) }
                    val pdf = android.graphics.pdf.PdfDocument()
                    try {
                        val page = pdf.startPage(android.graphics.pdf.PdfDocument.PageInfo.Builder(pw, ph, 1).create())
                        page.canvas.drawColor(android.graphics.Color.WHITE)
                        page.canvas.drawBitmap(
                            bmp,
                            null,
                            android.graphics.RectF(margin.toFloat(), margin.toFloat(), (pw - margin).toFloat(), (ph - margin).toFloat()),
                            android.graphics.Paint(android.graphics.Paint.FILTER_BITMAP_FLAG),
                        )
                        pdf.finishPage(page)
                        app.contentResolver.openOutputStream(dest, "wt")?.use { pdf.writeTo(it) } ?: error("cannot write")
                    } finally {
                        pdf.close()
                        bmp.recycle()
                    }
                }
            }
            error = result.exceptionOrNull()?.let { app.getString(R.string.save_failed, it.message ?: "") }
            if (result.isSuccess) {
                android.widget.Toast.makeText(app, R.string.saved, android.widget.Toast.LENGTH_SHORT).show()
            }
        }
    }

    fun toggleBackground() {
        val s = state ?: return
        state = s.copy(dark = !s.dark)
        renderedFor = null
        render()
    }

    fun setLayerVisible(name: String, visible: Boolean) {
        viewModelScope.launch {
            val layers = withContext(engine) {
                runCatching {
                    doc?.setLayerVisible(name, visible)
                    doc?.layers()
                }.getOrNull()
            } ?: return@launch
            state = state?.copy(layers = layers)
            renderedFor = null
            render()
        }
    }

    /** All of a document's bytes, read into one array of the file's size when the provider tells it. */
    private fun readAll(resolver: android.content.ContentResolver, uri: Uri, size: Long): ByteArray {
        val input = resolver.openInputStream(uri) ?: error("no data")
        return input.use { stream ->
            if (size <= 0 || size > Int.MAX_VALUE - 8) return@use stream.readBytes()
            val bytes = ByteArray(size.toInt())
            var read = 0
            while (read < bytes.size) {
                val n = stream.read(bytes, read, bytes.size - read)
                if (n < 0) break
                read += n
            }
            if (read < bytes.size) return@use bytes.copyOf(read)
            // A provider whose size was too small: the rest is read on.
            val next = stream.read()
            if (next < 0) return@use bytes
            val all = java.io.ByteArrayOutputStream(bytes.size * 2)
            all.write(bytes)
            all.write(next)
            stream.copyTo(all)
            all.toByteArray()
        }
    }

    override fun onCleared() {
        // viewModelScope is already cancelled here; close on the engine thread after any render in flight.
        CoroutineScope(engine).launch { doc?.close() }
    }
}
