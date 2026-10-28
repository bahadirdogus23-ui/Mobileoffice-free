package org.mobileofficefree.app

import android.app.Application
import android.graphics.Bitmap
import android.net.Uri
import android.util.LruCache
import androidx.compose.runtime.getValue
import androidx.core.graphics.createBitmap
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import java.nio.ByteBuffer
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.mobileofficefree.engine.WordDocument

data class WordUiState(
    val uri: Uri,
    val fileName: String,
    /** Page sizes in points. */
    val pages: List<Pair<Float, Float>>,
)

class WordViewModel(app: Application) : AndroidViewModel(app) {

    /** One engine thread per document type (see AGENTS.md). */
    @OptIn(ExperimentalCoroutinesApi::class)
    private val engine: CoroutineDispatcher = Dispatchers.IO.limitedParallelism(1)

    private var doc: WordDocument? = null
    private val recents = RecentFiles(app)

    var state by mutableStateOf<WordUiState?>(null)
        private set
    var loading by mutableStateOf(false)
        private set
    var error by mutableStateOf<String?>(null)
        private set

    /** Rendered pages by index, at the width last asked for. Bitmaps leave memory via [cache]. */
    private val shown = mutableStateMapOf<Int, Bitmap>()
    private val pending = HashSet<Int>()
    private var renderWidth = 0
    private val cache = object : LruCache<Int, Bitmap>(cacheBytes()) {
        override fun sizeOf(key: Int, value: Bitmap) = value.byteCount
        override fun entryRemoved(evicted: Boolean, key: Int, old: Bitmap, new: Bitmap?) {
            if (evicted) shown.remove(key)
        }
    }

    fun open(uri: Uri) {
        val resolver = getApplication<Application>().contentResolver
        loading = true
        viewModelScope.launch {
            val result = withContext(engine) {
                runCatching {
                    val info = FileInfo.of(resolver, uri)
                    val bytes = resolver.openInputStream(uri)?.use { it.readBytes() } ?: error("no data")
                    val next = WordDocument.open(info.name, bytes, null)
                    doc?.close()
                    doc = next
                    val sizes = (0u until next.pageCount()).map { next.pageSize(it).let { s -> s.width to s.height } }
                    recents.remember(uri, info.name, DocumentKind.WORD, info.size)
                    WordUiState(uri, info.name, sizes)
                }
            }
            loading = false
            clearPages()
            result.fold(
                onSuccess = { state = it },
                onFailure = { error = it.message ?: it.javaClass.simpleName },
            )
        }
    }

    fun close() {
        val old = doc
        doc = null
        state = null
        clearPages()
        viewModelScope.launch(engine) { old?.close() }
    }

    fun consumeError() {
        error = null
    }

    /** Remembers how far the person scrolled, for the "viewed %" badge on the home screen. */
    fun reportPosition(firstVisible: Int) {
        val s = state ?: return
        val pct = ((firstVisible + 1) * 100) / s.pages.size.coerceAtLeast(1)
        recents.setProgress(s.uri, pct)
    }

    /** The page bitmap at [widthPx] if ready; otherwise starts rendering it and returns null. */
    fun page(index: Int, widthPx: Int): Bitmap? {
        if (widthPx <= 0) return null
        if (widthPx != renderWidth) {
            renderWidth = widthPx
            clearPages()
        }
        shown[index]?.let { return it }
        if (pending.add(index)) {
            viewModelScope.launch {
                val bmp = withContext(engine) { runCatching { render(index, widthPx) }.getOrNull() }
                pending.remove(index)
                if (bmp != null && widthPx == renderWidth) {
                    cache.put(index, bmp)
                    shown[index] = bmp
                }
            }
        }
        return null
    }

    /** Runs on the engine thread. */
    private fun render(index: Int, widthPx: Int): Bitmap? {
        val d = doc ?: return null
        val size = d.pageSize(index.toUInt())
        val raster = d.renderPage(index.toUInt(), widthPx / size.width)
        // The engine's premultiplied RGBA is exactly ARGB_8888's memory layout.
        return createBitmap(raster.width.toInt(), raster.height.toInt()).apply {
            copyPixelsFromBuffer(ByteBuffer.wrap(raster.pixels))
        }
    }

    private fun clearPages() {
        cache.evictAll()
        shown.clear()
        pending.clear()
    }

    override fun onCleared() {
        doc?.close()
        doc = null
    }

    private companion object {
        /** An eighth of the heap for page pictures. */
        fun cacheBytes(): Int = (Runtime.getRuntime().maxMemory() / 8).toInt()
    }
}
