package org.mobileofficefree.app

import android.app.Application
import android.graphics.Bitmap
import android.graphics.Color
import android.graphics.pdf.PdfRenderer
import android.net.Uri
import android.os.ParcelFileDescriptor
import androidx.core.graphics.createBitmap

/**
 * PDF viewing through Android's own PdfRenderer (PDFium inside, nothing added to the app);
 * see docs/adr/0003-pdf-goruntuleme.md. Editing tools will come from the PdfCraft engine.
 */
class PdfViewModel(app: Application) : PagedViewModel(app, DocumentKind.PDF) {
    private var fd: ParcelFileDescriptor? = null
    private var renderer: PdfRenderer? = null

    override fun load(uri: Uri, name: String): List<Pair<Float, Float>> {
        val pfd = getApplication<Application>().contentResolver.openFileDescriptor(uri, "r") ?: error("no data")
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

    override val printsOriginal = true

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

    override fun release() {
        renderer?.close()
        renderer = null
        fd?.close()
        fd = null
    }
}
