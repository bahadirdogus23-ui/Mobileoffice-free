package org.mobileofficefree.app

import android.app.Application
import android.graphics.Bitmap
import android.graphics.Color
import android.graphics.pdf.PdfRenderer
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.text.format.Formatter
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.core.graphics.createBitmap
import androidx.lifecycle.viewModelScope
import java.io.File
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.security.SecureRandom
import org.mobileofficefree.engine.FormFieldKind
import org.mobileofficefree.engine.MoException
import org.mobileofficefree.engine.PdfDocument
import org.mobileofficefree.engine.PdfFormField
import org.mobileofficefree.engine.SensitiveKind

/**
 * PDF viewing through Android's own PdfRenderer (PDFium inside, nothing added to the app);
 * see docs/adr/0003-pdf-goruntuleme.md. Page tools (rotate, delete, combine, extract) go through
 * the PdfCraft engine: each edit writes the whole document to a cache file that the renderer
 * then shows; the opened file is never written, the person saves to a new one.
 *
 * Password-protected files are opened by the engine and shown from a copy without the password
 * kept in the app's cache (PdfRenderer cannot open them); what the person saves keeps it.
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

    /** A file waiting for its password, and whether the last one tried was wrong. */
    var locked by mutableStateOf<Uri?>(null)
        private set
    var wrongPassword by mutableStateOf(false)
        private set
    private var password: String? = null

    /** The file (as it would be saved) asks for a password. */
    var isProtected by mutableStateOf(false)
        private set
    /** The owner allows no changes with the password it was opened with. */
    var restricted by mutableStateOf(false)
        private set

    override val canEditPages: Boolean get() = !restricted
    override val hasUnsavedChanges: Boolean get() = edited
    override val printsOriginal: Boolean get() = !edited && !isProtected

    /** Tries [locked] again with [pass]. */
    fun unlock(pass: String) {
        val uri = locked ?: return
        locked = null
        password = pass
        open(uri)
    }

    fun cancelUnlock() {
        locked = null
        password = null
    }

    override fun openFailed(uri: Uri, e: Throwable): Boolean {
        if (e !is MoException.NeedsPassword && e !is MoException.WrongPassword) return false
        wrongPassword = e is MoException.WrongPassword
        locked = uri
        return true
    }

    /** Asks [pass] for opening the saved file (AES-256). */
    fun protect(pass: String) = edit {
        it.protect(pass, ByteArray(32).also(SecureRandom()::nextBytes))
        isProtected = it.isProtected()
    }

    /** The saved file opens without a password. */
    fun unprotect() = edit {
        it.unprotect()
        isProtected = it.isProtected()
    }

    fun rotatePages(pages: List<Int>, degrees: Int) = edit { it.rotate(pages.map(Int::toUInt), degrees) }

    fun deletePages(pages: List<Int>) = edit {
        it.delete(pages.map(Int::toUInt))
        added = 0 // the engine forgets what it added once pages go
    }

    /** Puts a hand-drawn signature on [page]; strokes are flat `x, y` fractions of the displayed page. */
    fun sign(page: Int, strokes: List<List<Double>>) = edit {
        it.sign(page.toUInt(), strokes)
        added++
    }

    /** Signatures, drawings and notes added that [undoAnnotation] can still take back. */
    var added by mutableIntStateOf(0)
        private set

    /** Writes pen and highlighter strokes into the PDF. */
    fun draw(strokes: List<PenStroke>) = edit { doc ->
        for (s in strokes) {
            doc.draw(s.page.toUInt(), listOf(s.flat()), s.pen)
            added++
        }
    }

    /** A sticky note with [text] at ([x], [y]), fractions of page [page]. */
    fun note(page: Int, x: Float, y: Float, text: String) = edit {
        it.note(page.toUInt(), x.toDouble(), y.toDouble(), text)
        added++
    }

    /** Takes back the last signature, drawing or note added. */
    fun undoAnnotation() = edit {
        if (it.undoAnnotation()) added--
    }

    /** Appends every page of the PDF at [uri] (combining files). */
    fun appendPdf(uri: Uri) = edit { doc ->
        val bytes = getApplication<Application>().contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: error("no data")
        doc.append(bytes, null)
    }

    /** Writes a new PDF of just [pages] into [dest]. */
    fun extractTo(dest: Uri, pages: List<Int>) = write(dest) { it.extract(pages.map(Int::toUInt)) }

    /** A smaller copy waiting for the file it goes into, with the sizes before and after. */
    class Compressed(val bytes: ByteArray, val before: Long, val after: Long)

    var compressed by mutableStateOf<Compressed?>(null)
        private set

    /** Makes a smaller copy into [compressed] (pictures to 96 ppi [forScreen], else 150). */
    fun compress(forScreen: Boolean) {
        val app = getApplication<Application>()
        busy = true
        viewModelScope.launch {
            val result = withContext(engine) {
                runCatching {
                    val doc = editor()
                    val out = doc.compress(forScreen)
                    val uri = source ?: error("no document")
                    val before = if (edited) doc.save().size.toLong() else FileInfo.of(app.contentResolver, uri).size
                    Compressed(out, before, out.size.toLong())
                }
            }
            busy = false
            result.fold(
                onSuccess = {
                    // Not worth a new file unless it is clearly smaller.
                    if (it.before > 0 && it.after > it.before * 0.95) tell(app.getString(R.string.compress_no_gain)) else compressed = it
                },
                onFailure = { tell(app.getString(R.string.pdf_edit_failed, it.message ?: it.javaClass.simpleName)) },
            )
        }
    }

    /** Writes [compressed] into [dest] (null: the person cancelled). */
    fun saveCompressed(dest: Uri?) {
        val c = compressed ?: return
        compressed = null
        if (dest == null) return
        val app = getApplication<Application>()
        viewModelScope.launch {
            val result = withContext(engine) {
                runCatching { app.contentResolver.openOutputStream(dest, "wt")?.use { it.write(c.bytes) } ?: error("cannot write") }
            }
            tell(
                result.fold(
                    onSuccess = { app.getString(R.string.compress_saved, Formatter.formatShortFileSize(app, c.before), Formatter.formatShortFileSize(app, c.after)) },
                    onFailure = { app.getString(R.string.save_failed, it.message ?: "") },
                ),
            )
        }
    }

    /** The form's fields once asked for ([loadForm]); empty when the PDF has no form. */
    var formFields by mutableStateOf<List<PdfFormField>?>(null)
        private set

    fun loadForm() {
        val app = getApplication<Application>()
        busy = true
        viewModelScope.launch {
            val result = withContext(engine) { runCatching { editor().fields() } }
            busy = false
            result.fold(
                onSuccess = {
                    if (it.none { f -> f.kind != FormFieldKind.OTHER }) tell(app.getString(R.string.form_none))
                    formFields = it
                },
                onFailure = { tell(app.getString(R.string.pdf_edit_failed, it.message ?: it.javaClass.simpleName)) },
            )
        }
    }

    fun closeForm() {
        formFields = null
    }

    /** Writes what the person typed and picked into the form. */
    fun fillForm(changes: Map<String, FieldInput>) = edit { doc ->
        for ((name, input) in changes) {
            when (input) {
                is FieldInput.Text -> doc.setText(name, input.text)
                is FieldInput.Check -> doc.setCheck(name, input.on)
                is FieldInput.Choice -> doc.setChoice(name, input.value)
            }
        }
        formFields = null
    }

    /** Writes the edited document into the new file [dest]. */
    fun saveTo(dest: Uri) = write(dest, done = { edited = false }) { it.save() }

    private fun editor(): PdfDocument = editor ?: run {
        val uri = source ?: error("no document")
        val bytes = getApplication<Application>().contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: error("no data")
        PdfDocument.open(bytes, null).also {
            editor = it
            isProtected = it.isProtected()
            restricted = it.isRestricted()
        }
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
                    file.writeBytes(doc.displayCopy())
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
        isProtected = false
        restricted = false
        val app = getApplication<Application>()
        val pfd = app.contentResolver.openFileDescriptor(uri, "r") ?: error("no data")
        return try {
            openRenderer(pfd)
        } catch (_: SecurityException) {
            // Encrypted: the engine opens it (asking for the password) and gives a copy to show.
            val pass = password
            password = null
            val bytes = app.contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: error("no data")
            val doc = PdfDocument.open(bytes, pass)
            editor = doc
            isProtected = doc.isProtected()
            restricted = doc.isRestricted()
            val dir = File(app.cacheDir, "pdf-edit").apply { mkdirs() }
            val file = File(dir, "edit-${++editCount}.pdf")
            file.writeBytes(doc.displayCopy())
            openRenderer(ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY))
        }
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

    /**
     * Search and reading aloud go through the engine's text layer: every Android version, and
     * Turkish ı/İ and accents match their plain letters.
     */
    override val searchable = true

    override fun findText(query: String): List<FoundRect> {
        val sizes = state?.pages ?: return emptyList()
        return editor().findText(query, MAX_MATCHES.toUInt()).flatMapIndexed { index, hit ->
            val (w, h) = sizes.getOrNull(hit.page.toInt()) ?: return@flatMapIndexed emptyList()
            hit.rects.map { r -> FoundRect(index, hit.page.toInt(), r.left * w, r.top * h, r.right * w, r.bottom * h) }
        }
    }

    override val canReadAloud = true

    override val exportFormats = PDF_EXPORT_FORMATS

    override fun export(ext: String, name: String): ByteArray = when (ext) {
        "txt" -> pageTexts().joinToString("\n\n").toByteArray(Charsets.UTF_8)
        "zip" -> pagesAsJpegs(name.substringBeforeLast('.'))
        else -> error("cannot convert")
    }

    /** Every page drawn at [EXPORT_DPI] as a JPEG, in a ZIP (one picture per page). */
    private fun pagesAsJpegs(stem: String): ByteArray {
        val sizes = state?.pages ?: error("no document")
        val out = java.io.ByteArrayOutputStream()
        java.util.zip.ZipOutputStream(out).use { zip ->
            sizes.forEachIndexed { i, (w, _) ->
                val px = (w * EXPORT_DPI / 72f).toInt().coerceIn(16, MAX_EXPORT_SIDE)
                val bmp = renderPage(i, px, false) ?: return@forEachIndexed
                try {
                    zip.putNextEntry(java.util.zip.ZipEntry("%s-%03d.jpg".format(java.util.Locale.ROOT, stem, i + 1)))
                    bmp.compress(Bitmap.CompressFormat.JPEG, 88, zip)
                    zip.closeEntry()
                } finally {
                    bmp.recycle()
                }
            }
        }
        return out.toByteArray()
    }

    override fun pageTexts(): List<String> {
        val doc = editor()
        return (0u until doc.pageCount()).map { runCatching { doc.pageText(it) }.getOrDefault("") }
    }

    /** Black-out marks waiting to be applied. */
    var redactionMarks by mutableIntStateOf(0)
        private set

    /** Marks every place of [kinds] and [words] for blacking out. */
    fun markRedactions(kinds: List<SensitiveKind>, words: List<String>) = edit { doc ->
        val found = doc.markRedactions(kinds, words).size
        redactionMarks = doc.redactionMarks().toInt()
        tell(getApplication<Application>().resources.getQuantityString(R.plurals.redact_marked, found, found))
    }

    /** Drops the marks not yet applied. */
    fun clearRedactionMarks() = edit { doc ->
        doc.clearRedactionMarks()
        redactionMarks = 0
        added = 0
    }

    /** Removes what is under every mark for good (in the copy being edited). */
    fun applyRedactions() = edit { doc ->
        val n = doc.applyRedactions().toInt()
        redactionMarks = 0
        added = 0
        tell(getApplication<Application>().resources.getQuantityString(R.plurals.redact_applied, n, n))
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
        viewModelScope.launch {
            edited = false
            added = 0
            redactionMarks = 0
        }
    }
}

private const val MAX_MATCHES = 2_000
/** Pictures of pages: sharp enough to read and print, small enough to send. */
private const val EXPORT_DPI = 150f
private const val MAX_EXPORT_SIDE = 3_000
