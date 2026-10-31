package org.mobileofficefree.app

import android.app.Application
import android.graphics.Bitmap
import android.net.Uri
import android.os.SystemClock
import android.util.LruCache
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** One rectangle of search match [index] on page [page], in page points. */
data class FoundRect(val index: Int, val page: Int, val left: Float, val top: Float, val right: Float, val bottom: Float)

data class PagedUiState(
    val uri: Uri,
    val fileName: String,
    /** Page sizes in points. */
    val pages: List<Pair<Float, Float>>,
    /** Counts edits, so pages are drawn again after one even when their sizes stay the same. */
    val edits: Int = 0,
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

    /**
     * Rendered pages by index. After a zoom or a night-mode change a page keeps showing its old
     * picture until the new one is ready, so nothing flashes white. Bitmaps leave memory via [cache].
     */
    private val shown = mutableStateMapOf<Int, Bitmap>()
    /** Pages in [shown] drawn for the current width and colours. */
    private val fresh = HashSet<Int>()
    private val pending = HashSet<Int>()
    private var renderWidth = 0
    /** Bumped whenever what a page should look like changes; renders for an older one are dropped. */
    private var generation = 0

    private var renderNight = false
    /** When the open document was ready, until its first page is drawn (for [Diagnostics]). */
    @Volatile private var firstPageFrom = 0L

    /** Pages can be rotated, deleted, combined… (the PDF viewer). */
    open val canEditPages: Boolean = false

    /** Edits not yet written to a file; closing asks first. */
    open val hasUnsavedChanges: Boolean get() = false

    var askClose by mutableStateOf(false)
        private set

    /** Back or the arrow: closes, asking first when there are unsaved edits. */
    fun requestClose() {
        if (hasUnsavedChanges) askClose = true else close()
    }

    /** The answer to "save the changes?": true closes (discarding), null stays. */
    fun answerClose(discard: Boolean?) {
        askClose = false
        if (discard == true) close()
    }

    /** After an edit changed the pages: show the new ones. Call on the main thread. */
    protected fun pagesChanged(pages: List<Pair<Float, Float>>) {
        state = state?.copy(pages = pages)
        clearPages()
        clearSearch()
    }

    /**
     * After a small edit (typing): the new page sizes; every page is drawn again but keeps its old
     * picture until the new one is ready, so the screen does not flash. Call on the main thread.
     */
    protected fun pagesEdited(pages: List<Pair<Float, Float>>) {
        val s = state ?: return
        state = s.copy(pages = pages, edits = s.edits + 1)
        invalidate()
        clearThumbnails()
        shown.keys.filter { it >= pages.size }.forEach { shown.remove(it) }
    }

    protected fun tell(text: String) {
        notice = text
    }

    /** Whether [renderPage] draws night pages itself (true) or the screen should invert its colours. */
    open val drawsNight: Boolean = false

    /** Whether printing sends the file itself (a PDF) rather than pages drawn by [renderPage]. */
    open val printsOriginal: Boolean = false
    /** Memory for page pictures. Since Android 8 bitmaps live outside the Java heap, so this follows the phone's RAM. */
    private val cacheBudget = cacheBytes(app)

    /**
     * The largest one page picture may be: a sixth of [cacheBudget], so the pages on screen always
     * fit together. (When they did not, they pushed each other out and the screen flickered.)
     */
    val maxPageBytes: Long get() = cacheBudget / 6L

    private val cache = object : LruCache<Int, Bitmap>(cacheBudget) {
        override fun sizeOf(key: Int, value: Bitmap) = value.byteCount
        override fun entryRemoved(evicted: Boolean, key: Int, old: Bitmap, new: Bitmap?) {
            if (evicted) {
                shown.remove(key)
                fresh.remove(key)
            }
        }
    }

    /** Whether [findText] can search this kind of document (on this Android version). */
    open val searchable: Boolean = false

    /** Every place [query] appears, at most a couple of thousand matches. Runs on [engine]. */
    protected open fun findText(query: String): List<FoundRect> = emptyList()

    var query by mutableStateOf("")
        private set
    var found by mutableStateOf<List<FoundRect>>(emptyList())
        private set
    var matchCount by mutableIntStateOf(0)
        private set
    var currentMatch by mutableIntStateOf(0)
        private set
    var searching by mutableStateOf(false)
        private set
    private var searchJob: Job? = null

    /** Searches as the person types, a moment after they stop. */
    fun search(text: String) {
        query = text
        searchJob?.cancel()
        if (text.isBlank()) {
            clearSearch(keepQuery = true)
            return
        }
        searchJob = viewModelScope.launch {
            delay(300)
            searching = true
            val hits = withContext(engine) { runCatching { findText(text.trim()) }.getOrDefault(emptyList()) }
            found = hits
            matchCount = (hits.maxOfOrNull { it.index } ?: -1) + 1
            currentMatch = 0
            searching = false
        }
    }

    fun nextMatch(step: Int) {
        if (matchCount == 0) return
        currentMatch = Math.floorMod(currentMatch + step, matchCount)
    }

    private fun clearSearch(keepQuery: Boolean = false) {
        searchJob?.cancel()
        if (!keepQuery) query = ""
        found = emptyList()
        matchCount = 0
        currentMatch = 0
        searching = false
    }

    /** What this document can be converted to ("Dönüştür"); empty hides the button. */
    open val exportFormats: List<ExportFormat> = emptyList()

    /** The document in format [ext]. Runs on [engine]. */
    protected open fun export(ext: String, name: String): ByteArray = error("cannot convert")

    /** One-off messages for the person (saved, failed). */
    var notice by mutableStateOf<String?>(null)
        private set

    fun consumeNotice() {
        notice = null
    }

    /** Writes the document converted to [format] into the new file [uri]; the original is not touched. */
    fun exportTo(uri: Uri, format: ExportFormat) {
        val app = getApplication<Application>()
        val name = state?.fileName ?: return
        viewModelScope.launch {
            val started = SystemClock.elapsedRealtime()
            val result = withContext(engine) {
                runCatching {
                    val bytes = export(format.ext, convertedName(name, format.ext))
                    // "wt" truncates: without it some providers leave an old file's tail behind.
                    app.contentResolver.openOutputStream(uri, "wt")?.use { it.write(bytes) } ?: error("cannot write")
                }
            }
            if (result.isSuccess) Diagnostics.record("${kind.name} → ${format.ext}", started)
            notice = result.fold(
                onSuccess = { app.getString(R.string.saved) },
                onFailure = { app.getString(R.string.save_failed, it.message ?: it.javaClass.simpleName) },
            )
        }
    }

    /** Whether [pageTexts] gives text to read aloud. */
    open val canReadAloud: Boolean = false

    /** The text starting on each page. Runs on [engine]. */
    protected open fun pageTexts(): List<String> = emptyList()

    /** The page being read aloud, or -1 when not reading. */
    var readingPage by mutableIntStateOf(-1)
        private set
    var readAloudProblem by mutableStateOf<ReadAloud.Problem?>(null)
        private set
    private var voice: ReadAloud? = null

    fun startReading(fromPage: Int) {
        readAloudProblem = null
        readingPage = fromPage
        val v = voice ?: ReadAloud(
            getApplication<Application>(),
            onPage = { readingPage = it },
            onDone = { readingPage = -1 },
            onProblem = {
                readAloudProblem = it
                readingPage = -1
            },
        ).also { voice = it }
        viewModelScope.launch {
            val texts = withContext(engine) { runCatching { pageTexts() }.getOrDefault(emptyList()) }
            if (readingPage >= 0) v.speak(texts, fromPage)
        }
    }

    fun stopReading() {
        voice?.stop()
        readingPage = -1
    }

    fun dismissReadAloudProblem() {
        readAloudProblem = null
    }

    /** Something to tell the person once the document is open (an unverified e-signature…). Runs on [engine]. */
    protected open fun openedNotice(): String? = null

    /** Opens [uri] (named [name]) and returns its page sizes in points. Runs on [engine]. */
    protected abstract fun load(uri: Uri, name: String): List<Pair<Float, Float>>

    /** Draws page [index] [widthPx] pixels wide, dark for [night] (only asked when [drawsNight]). Runs on [engine]. */
    protected abstract fun renderPage(index: Int, widthPx: Int, night: Boolean): Bitmap?

    /** Releases the open document. Runs on [engine]. */
    protected abstract fun release()

    fun open(uri: Uri) {
        val resolver = getApplication<Application>().contentResolver
        loading = true
        val started = SystemClock.elapsedRealtime()
        viewModelScope.launch {
            val result = withContext(engine) {
                runCatching {
                    release()
                    val info = FileInfo.of(resolver, uri)
                    val pages = load(uri, info.typedName)
                    Diagnostics.record("${kind.name} ${info.typedName.substringAfterLast('.')} açma (${pages.size} sayfa, ${sizeLabel(info.size)})", started)
                    firstPageFrom = SystemClock.elapsedRealtime()
                    recents.remember(uri, info.name, kind, info.size)
                    PagedUiState(uri, info.name, pages) to openedNotice()
                }
            }
            loading = false
            clearPages()
            clearSearch()
            result.fold(
                onSuccess = { (opened, note) ->
                    state = opened
                    notice = note
                },
                onFailure = { if (!openFailed(uri, it)) error = it.message ?: it.javaClass.simpleName },
            )
        }
    }

    /** A chance to answer a failed [open] (asking for a password…); true when handled. Main thread. */
    protected open fun openFailed(uri: Uri, e: Throwable): Boolean = false

    fun close() {
        stopReading()
        state = null
        clearSearch()
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

    /**
     * The picture for page [index]: drawn [widthPx] wide (dark for [night], if this engine draws
     * night pages) if ready, else an older picture of it, or null, while the new one is drawn.
     */
    fun page(index: Int, widthPx: Int, night: Boolean): Bitmap? {
        if (widthPx <= 0) return null
        val dark = night && drawsNight
        if (widthPx != renderWidth || dark != renderNight) {
            renderWidth = widthPx
            renderNight = dark
            invalidate()
        }
        if (index in fresh) return shown[index]
        if (pending.add(index)) {
            val gen = generation
            viewModelScope.launch {
                val bmp = withContext(engine) { runCatching { renderPage(index, widthPx, dark) }.getOrNull() }
                if (gen != generation) return@launch
                pending.remove(index)
                if (bmp != null) {
                    if (index == 0 && firstPageFrom != 0L) {
                        Diagnostics.record("${kind.name} ilk sayfa ${bmp.width}px", firstPageFrom)
                        firstPageFrom = 0L
                    }
                    cache.put(index, bmp)
                    shown[index] = bmp
                    fresh += index
                }
            }
        }
        return shown[index]
    }

    /** Page [index] drawn [widthPx] wide in normal colours, for printing; the caller owns the bitmap. */
    suspend fun renderForPrint(index: Int, widthPx: Int): Bitmap? = withContext(engine) { renderPage(index, widthPx, false) }

    /** Small page pictures for the "Pages" panel, kept apart from the full-size ones. */
    private val thumbs = mutableStateMapOf<Int, Bitmap>()
    private val thumbsPending = HashSet<Int>()
    private val thumbCache = object : LruCache<Int, Bitmap>(THUMB_CACHE_BYTES) {
        override fun sizeOf(key: Int, value: Bitmap) = value.byteCount
        override fun entryRemoved(evicted: Boolean, key: Int, old: Bitmap, new: Bitmap?) {
            if (evicted) thumbs.remove(key)
        }
    }

    /** Page [index] [THUMB_WIDTH] pixels wide if drawn; otherwise starts drawing it and returns null. */
    fun thumbnail(index: Int): Bitmap? {
        thumbs[index]?.let { return it }
        if (thumbsPending.add(index)) {
            val doc = state?.uri
            viewModelScope.launch {
                val bmp = withContext(engine) { runCatching { renderPage(index, THUMB_WIDTH, false) }.getOrNull() }
                thumbsPending.remove(index)
                if (bmp != null && doc == state?.uri) {
                    thumbCache.put(index, bmp)
                    thumbs[index] = bmp
                }
            }
        }
        return null
    }

    private fun clearThumbnails() {
        thumbCache.evictAll()
        thumbs.clear()
        thumbsPending.clear()
    }

    /** Marks every page out of date; their old pictures stay until the new ones arrive. */
    private fun invalidate() {
        generation++
        fresh.clear()
        pending.clear()
    }

    private fun clearPages() {
        invalidate()
        clearThumbnails()
        cache.evictAll()
        shown.clear()
    }

    override fun onCleared() {
        voice?.shutdown()
        // The engine thread may be busy; release on it so a render in progress finishes first.
        // viewModelScope is already cancelled here, so this runs in a scope of its own.
        CoroutineScope(engine).launch { release() }
    }

    private companion object {
        const val THUMB_WIDTH = 240
        const val THUMB_CACHE_BYTES = 24 shl 20

        /**
         * A 24th of the phone's RAM, between 48 and 256 MB: on an 11 GB phone a twelfth (384 MB)
         * pushed the app to 470 MB while scrolling a long PDF, more than reading needs.
         */
        fun cacheBytes(app: Application): Int {
            val am = app.getSystemService(android.app.ActivityManager::class.java)
            val ram = android.app.ActivityManager.MemoryInfo().also { am?.getMemoryInfo(it) }.totalMem
            return (ram / 24).coerceIn(48L shl 20, 256L shl 20).toInt()
        }
    }
}

internal fun sizeLabel(bytes: Long) = if (bytes < 0) "?" else "%.1f MB".format(java.util.Locale.ROOT, bytes / (1024.0 * 1024.0))
