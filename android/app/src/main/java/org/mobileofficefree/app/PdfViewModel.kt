package org.mobileofficefree.app

import android.app.Application
import android.graphics.Bitmap
import android.graphics.Color
import android.graphics.pdf.PdfRenderer
import android.net.Uri
import android.os.Build
import android.os.ParcelFileDescriptor
import androidx.annotation.RequiresApi
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

    override fun release() {
        renderer?.close()
        renderer = null
        fd?.close()
        fd = null
    }
}

private const val MAX_MATCHES = 2_000
