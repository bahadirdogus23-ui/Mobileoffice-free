package org.mobileofficefree.app

import android.app.Application
import android.graphics.Bitmap
import android.net.Uri
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

    fun open(uri: Uri) {
        val resolver = getApplication<Application>().contentResolver
        loading = true
        viewModelScope.launch {
            val result = withContext(engine) {
                runCatching {
                    doc?.close()
                    doc = null
                    val info = FileInfo.of(resolver, uri)
                    val bytes = resolver.openInputStream(uri)?.use { it.readBytes() } ?: error("no data")
                    val next = CadDocument.open(info.typedName, bytes)
                    doc = next
                    recents.remember(uri, info.name, DocumentKind.CAD, info.size)
                    CadUiState(uri, info.name, next.spaceNames(), 0, next.layers(), dark = true, notes = next.notes())
                }
            }
            loading = false
            frame = null
            view = null
            result.fold(onSuccess = { state = it; fit() }, onFailure = { error = it.message ?: it.javaClass.simpleName })
        }
    }

    fun close() {
        state = null
        frame = null
        view = null
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

    /** Draws the current view sharply (called when a gesture settles). */
    fun render() {
        val s = state ?: return
        val v = view ?: return
        val (w, h) = viewport
        if (w <= 0 || renderedFor == (v to viewport)) return
        renderedFor = v to viewport
        viewModelScope.launch {
            val bmp = withContext(engine) {
                runCatching {
                    val r = doc?.render(s.space.toUInt(), v.centerX, v.centerY, v.scale, w.toUInt(), h.toUInt(), s.dark)
                        ?: return@runCatching null
                    // tiny-skia's premultiplied RGBA is exactly ARGB_8888's memory layout.
                    createBitmap(r.width.toInt(), r.height.toInt()).apply { copyPixelsFromBuffer(ByteBuffer.wrap(r.pixels)) }
                }.getOrNull()
            }
            if (bmp != null) frame = CadFrame(bmp, v)
        }
    }

    fun selectSpace(index: Int) {
        val s = state ?: return
        state = s.copy(space = index)
        renderedFor = null
        fit()
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

    override fun onCleared() {
        // viewModelScope is already cancelled here; close on the engine thread after any render in flight.
        CoroutineScope(engine).launch { doc?.close() }
    }
}
