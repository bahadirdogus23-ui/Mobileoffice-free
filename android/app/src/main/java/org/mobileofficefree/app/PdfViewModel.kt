package org.mobileofficefree.app

import android.app.Application
import android.graphics.Bitmap
import android.graphics.Color
import android.graphics.pdf.PdfRenderer
import android.net.Uri
import android.os.Build
import android.os.ParcelFileDescriptor
import androidx.annotation.RequiresApi
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.core.graphics.createBitmap
import androidx.lifecycle.viewModelScope
import java.io.File
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.mobileofficefree.engine.PdfDocument

/**
 * PDF viewing through Android's own PdfRenderer (PDFium inside, nothing added to the app);
 * see docs/adr/0003-pdf-goruntuleme.md. Page tools (rotate, delete, combine, extract) go through
 * the PdfCraft engine: each edit writes the whole document to a cache file that the renderer
 * then shows; the opened file is never written, the person saves to a new one.
 */
class PdfViewModel(app: Application) : PagedViewModel(app, DocumentKind.PDF) {
    private var fd: ParcelFileDescriptor? = null
    private var renderer: PdfRenderer? = null

    /** The PdfCraft document, made on the first edit. Engine thread only. */
    private var editor: PdfDocument? = null
    private var source: Uri? = null
    private var editCount = 0

    var edited by mutableStateOf(false)
        private set
    var busy by mutableStateOf(false)
        private set

    override val canEditPages = true
    override val hasUnsavedChanges: Boolean get() = edited
    override val printsOriginal: Boolean get() = !edited

    fun rotatePages(pages: List<Int>, degrees: Int) = edit { it.rotate(pages.map(Int::toUInt), degrees) }

    fun deletePages(pages: List<Int>) = edit { it.delete(pages.map(Int::toUInt)) }

    /** Puts a hand-drawn signature on [page]; strokes are flat `x, y` fractions of the displayed page. */
    fun sign(page: Int, strokes: List<List<Double>>) = edit { it.sign(page.toUInt(), strokes) }

    /** Appends every page of the PDF at [uri] (combining files). */
    fun appendPdf(uri: Uri) = edit { doc ->
        val bytes = getApplication<Application>().contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: error("no data")
        doc.append(bytes, null)
    }

    /** Writes a new PDF of just [pages] into [dest]. */
    fun extractTo(dest: Uri, pages: List<Int>) = write(dest) { it.extract(pages.map(Int::toUInt)) }

    /** Writes the edited document into the new file [dest]. */
    fun saveTo(dest: Uri) = write(dest, done = { edited = false }) { it.save() }

    private fun editor(): PdfDocument = editor ?: run {
        val uri = source ?: error("no document")
        val bytes = getApplication<Application>().contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: error("no data")
        PdfDocument.open(bytes, null).also { editor = it }
    }

    private fun edit(op: (PdfDocument) -> Unit) {
        val app = getApplication<Application>()
        busy = true
        viewModelScope.launch {
            val result = withContext(engine) {
                runCatching {
                    val doc = editor()
                    op(doc)
                    // A new cache file per edit: the renderer still has the previous one open.
                    val dir = File(app.cacheDir, "pdf-edit").apply { mkdirs() }
                    val file = File(dir, "edit-${++editCount}.pdf")
                    file.writeBytes(doc.save())
                    closeRenderer()
                    dir.listFiles()?.filter { it != file }?.forEach { it.delete() }
                    openRenderer(ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY))
                }
            }
            busy = false
            result.fold(
                onSuccess = {
                    edited = true
                    pagesChanged(it)
                },
                onFailure = { tell(app.getString(R.string.pdf_edit_failed, it.message ?: it.javaClass.simpleName)) },
            )
        }
    }

    private fun write(dest: Uri, done: () -> Unit = {}, bytes: (PdfDocument) -> ByteArray) {
        val app = getApplication<Application>()
        busy = true
        viewModelScope.launch {
            val result = withContext(engine) {
                runCatching {
                    val out = bytes(editor())
                    app.contentResolver.openOutputStream(dest, "wt")?.use { it.write(out) } ?: error("cannot write")
                }
            }
            busy = false
            if (result.isSuccess) done()
            tell(result.fold(onSuccess = { app.getString(R.string.saved) }, onFailure = { app.getString(R.string.save_failed, it.message ?: "") }))
        }
    }

    override fun load(uri: Uri, name: String): List<Pair<Float, Float>> {
        source = uri
        val pfd = getApplication<Application>().contentResolver.openFileDescriptor(uri, "r") ?: error("no data")
        return openRenderer(pfd)
    }

    private fun openRenderer(pfd: ParcelFileDescriptor): List<Pair<Float, Float>> {
        val r = try {
            PdfRenderer(pfd)
        } catch (e: Exception) {
            pfd.close()
            throw e
        }
        fd = pfd
        renderer = r
        // PdfRenderer reports page sizes in points (1/72 inch).
        return (0 until r.pageCount).map { i ->
            val page = r.openPage(i)
            try {
                page.width.toFloat() to page.height.toFloat()
            } finally {
                page.close()
            }
        }
    }


    override fun renderPage(index: Int, widthPx: Int, night: Boolean): Bitmap? {
        val r = renderer ?: return null
        val page = r.openPage(index)
        try {
            val height = (widthPx.toLong() * page.height / page.width).toInt().coerceAtLeast(1)
            return createBitmap(widthPx, height).apply {
                // Pages without a background are transparent; paper is white.
                eraseColor(Color.WHITE)
                page.render(this, null, null, PdfRenderer.Page.RENDER_MODE_FOR_DISPLAY)
            }
        } finally {
            page.close()
        }
    }

    /** PdfRenderer can search text from Android 15 on. */
    override val searchable = Build.VERSION.SDK_INT >= Build.VERSION_CODES.VANILLA_ICE_CREAM

    override fun findText(query: String): List<FoundRect> =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.VANILLA_ICE_CREAM) searchPages(query) else emptyList()

    @RequiresApi(Build.VERSION_CODES.VANILLA_ICE_CREAM)
    private fun searchPages(query: String): List<FoundRect> {
        val r = renderer ?: return emptyList()
        val out = ArrayList<FoundRect>()
        var index = 0
        for (i in 0 until r.pageCount) {
            val page = r.openPage(i)
            try {
                for (match in page.searchText(query)) {
                    // Bounds are in page points, like the page sizes.
                    for (b in match.bounds) out += FoundRect(index, i, b.left, b.top, b.right, b.bottom)
                    if (++index >= MAX_MATCHES) return out
                }
            } finally {
                page.close()
            }
        }
        return out
    }

    private fun closeRenderer() {
        renderer?.close()
        renderer = null
        fd?.close()
        fd = null
    }

    override fun release() {
        closeRenderer()
        editor?.close()
        editor = null
        source = null
        File(getApplication<Application>().cacheDir, "pdf-edit").deleteRecursively()
        viewModelScope.launch { edited = false }
    }
}

private const val MAX_MATCHES = 2_000
