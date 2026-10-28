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

    override fun renderPage(index: Int, widthPx: Int): Bitmap? {
        val d = doc ?: return null
        val size = d.pageSize(index.toUInt())
        val raster = d.renderPage(index.toUInt(), widthPx / size.width)
        // The engine's premultiplied RGBA is exactly ARGB_8888's memory layout.
        return createBitmap(raster.width.toInt(), raster.height.toInt()).apply {
            copyPixelsFromBuffer(ByteBuffer.wrap(raster.pixels))
        }
    }

    override fun release() {
        doc?.close()
        doc = null
    }
}
