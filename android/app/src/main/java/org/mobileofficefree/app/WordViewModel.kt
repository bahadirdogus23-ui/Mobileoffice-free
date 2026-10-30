package org.mobileofficefree.app

import android.app.Application
import android.graphics.Bitmap
import android.net.Uri
import androidx.core.graphics.createBitmap
import java.nio.ByteBuffer
import org.mobileofficefree.engine.WordDocument

/** Word-processing documents (DOCX, DOC, ODT, RTF, …) drawn by the WordCraft engine. */
class WordViewModel(app: Application) : PagedViewModel(app, DocumentKind.WORD) {
    private var doc: WordDocument? = null

    override fun load(uri: Uri, name: String): List<Pair<Float, Float>> {
        val bytes = getApplication<Application>().contentResolver.openInputStream(uri)?.use { it.readBytes() }
            ?: error("no data")
        val next = WordDocument.open(name, bytes, null)
        doc = next
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
    }
}
