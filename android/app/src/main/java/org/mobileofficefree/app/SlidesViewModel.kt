package org.mobileofficefree.app

import android.app.Application
import android.graphics.Bitmap
import android.net.Uri
import androidx.core.graphics.createBitmap
import java.nio.ByteBuffer
import org.mobileofficefree.engine.SlidesDocument

/** Presentations (PPTX, PPSX, POTX) drawn by the DeckCraft renderer, one slide per page. */
class SlidesViewModel(app: Application) : PagedViewModel(app, DocumentKind.SLIDES) {
    private var doc: SlidesDocument? = null

    override fun load(uri: Uri, name: String): List<Pair<Float, Float>> {
        val bytes = getApplication<Application>().contentResolver.openInputStream(uri)?.use { it.readBytes() }
            ?: error("no data")
        val next = SlidesDocument.open(bytes)
        doc = next
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

    override val exportFormats = listOf(PDF_FORMAT)

    override fun export(ext: String, name: String): ByteArray = doc?.exportPdf() ?: error("no document")

    override fun release() {
        doc?.close()
        doc = null
    }
}
