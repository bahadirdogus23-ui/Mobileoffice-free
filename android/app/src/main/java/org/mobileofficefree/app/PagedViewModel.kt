package org.mobileofficefree.app

import android.app.Application
import android.graphics.Bitmap
import android.net.Uri
import android.util.LruCache
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

data class PagedUiState(
    val uri: Uri,
    val fileName: String,
    /** Page sizes in points. */
    val pages: List<Pair<Float, Float>>,
)

/**
 * A document shown as pages one under another (Word, PDF). Subclasses open the file and draw one
 * page; this class keeps the bitmap cache, the reading progress and the single engine thread.
 */
abstract class PagedViewModel(app: Application, private val kind: DocumentKind) : AndroidViewModel(app) {

    /** Engines and PdfRenderer are not thread-safe: every call to them runs on this thread. */
    @OptIn(ExperimentalCoroutinesApi::class)
    protected val engine: CoroutineDispatcher = Dispatchers.IO.limitedParallelism(1)

    private val recents = RecentFiles(app)

    var state by mutableStateOf<PagedUiState?>(null)
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

    /** Opens [uri] (named [name]) and returns its page sizes in points. Runs on [engine]. */
    protected abstract fun load(uri: Uri, name: String): List<Pair<Float, Float>>

    /** Draws page [index] [widthPx] pixels wide. Runs on [engine]. */
    protected abstract fun renderPage(index: Int, widthPx: Int): Bitmap?

    /** Releases the open document. Runs on [engine]. */
    protected abstract fun release()

    fun open(uri: Uri) {
        val resolver = getApplication<Application>().contentResolver
        loading = true
        viewModelScope.launch {
            val result = withContext(engine) {
                runCatching {
                    release()
                    val info = FileInfo.of(resolver, uri)
                    val pages = load(uri, info.name)
                    recents.remember(uri, info.name, kind, info.size)
                    PagedUiState(uri, info.name, pages)
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
        state = null
        clearPages()
        viewModelScope.launch(engine) { release() }
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
                val bmp = withContext(engine) { runCatching { renderPage(index, widthPx) }.getOrNull() }
                pending.remove(index)
                if (bmp != null && widthPx == renderWidth) {
                    cache.put(index, bmp)
                    shown[index] = bmp
                }
            }
        }
        return null
    }

    private fun clearPages() {
        cache.evictAll()
        shown.clear()
        pending.clear()
    }

    override fun onCleared() {
        // The engine thread may be busy; release on it so a render in progress finishes first.
        // viewModelScope is already cancelled here, so this runs in a scope of its own.
        CoroutineScope(engine).launch { release() }
    }

    private companion object {
        /** An eighth of the heap for page pictures. */
        fun cacheBytes(): Int = (Runtime.getRuntime().maxMemory() / 8).toInt()
    }
}
