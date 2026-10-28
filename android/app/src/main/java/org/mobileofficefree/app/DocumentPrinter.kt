package org.mobileofficefree.app

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Paint
import android.graphics.Rect
import android.graphics.RectF
import android.graphics.pdf.PdfDocument
import android.net.Uri
import android.os.Bundle
import android.os.CancellationSignal
import android.os.ParcelFileDescriptor
import android.print.PageRange
import android.print.PrintAttributes
import android.print.PrintDocumentAdapter
import android.print.PrintDocumentInfo
import android.print.PrintManager
import androidx.lifecycle.viewModelScope
import java.io.FileOutputStream
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * Printing through Android's print service (any installed printer, or "Save as PDF").
 * PDFs go to the printer as they are; other documents are drawn page by page into a PDF.
 */
object DocumentPrinter {
    /** Print resolution for drawn pages: 2 pixels per point is 144 dpi, sharp on paper and light on memory. */
    private const val PIXELS_PER_POINT = 2f

    /** A4 in points, and the margin around a printed picture. */
    private const val A4_SHORT = 595
    private const val A4_LONG = 842
    private const val MARGIN = 24f

    fun print(context: Context, vm: PagedViewModel, state: PagedUiState) {
        start(context, state.fileName, state.pages.size, vm.viewModelScope) { dest, cancel ->
            if (vm.printsOriginal) {
                copy(context, state.uri, dest)
            } else {
                drawPages(dest, cancel, state.pages) { i, widthPx -> vm.renderForPrint(i, widthPx) }
            }
        }
    }

    /** One picture on one A4 page, turned to the picture's orientation and fitted inside a margin. */
    fun printImage(context: Context, scope: CoroutineScope, name: String, bitmap: Bitmap) {
        start(context, name, 1, scope) { dest, _ ->
            val landscape = bitmap.width > bitmap.height
            val (w, h) = if (landscape) A4_LONG to A4_SHORT else A4_SHORT to A4_LONG
            val pdf = PdfDocument()
            try {
                val page = pdf.startPage(PdfDocument.PageInfo.Builder(w, h, 1).create())
                val k = minOf((w - 2 * MARGIN) / bitmap.width, (h - 2 * MARGIN) / bitmap.height)
                val dw = bitmap.width * k
                val dh = bitmap.height * k
                val box = RectF((w - dw) / 2, (h - dh) / 2, (w + dw) / 2, (h + dh) / 2)
                page.canvas.drawBitmap(bitmap, null, box, Paint(Paint.FILTER_BITMAP_FLAG))
                pdf.finishPage(page)
                withContext(Dispatchers.IO) { FileOutputStream(dest.fileDescriptor).use { pdf.writeTo(it) } }
            } finally {
                pdf.close()
            }
        }
    }

    private fun start(
        context: Context,
        fileName: String,
        pageCount: Int,
        scope: CoroutineScope,
        write: suspend (ParcelFileDescriptor, CancellationSignal) -> Unit,
    ) {
        val manager = context.getSystemService(PrintManager::class.java) ?: return
        val jobName = fileName.substringBeforeLast('.')
        manager.print(jobName, Adapter(jobName, pageCount, scope, write), null)
    }

    private suspend fun copy(context: Context, uri: Uri, destination: ParcelFileDescriptor) = withContext(Dispatchers.IO) {
        val input = context.contentResolver.openInputStream(uri) ?: error("no data")
        input.use { src -> FileOutputStream(destination.fileDescriptor).use { src.copyTo(it) } }
    }

    private suspend fun drawPages(
        destination: ParcelFileDescriptor,
        cancel: CancellationSignal,
        pages: List<Pair<Float, Float>>,
        render: suspend (Int, Int) -> Bitmap?,
    ) {
        val pdf = PdfDocument()
        try {
            for ((i, size) in pages.withIndex()) {
                if (cancel.isCanceled) return
                val (w, h) = size
                val bmp = render(i, (w * PIXELS_PER_POINT).toInt()) ?: error("page ${i + 1}")
                val page = pdf.startPage(PdfDocument.PageInfo.Builder(w.toInt(), h.toInt(), i + 1).create())
                page.canvas.drawBitmap(bmp, null, Rect(0, 0, w.toInt(), h.toInt()), Paint(Paint.FILTER_BITMAP_FLAG))
                pdf.finishPage(page)
                bmp.recycle()
            }
            withContext(Dispatchers.IO) { FileOutputStream(destination.fileDescriptor).use { pdf.writeTo(it) } }
        } finally {
            pdf.close()
        }
    }

    private class Adapter(
        private val jobName: String,
        private val pageCount: Int,
        private val scope: CoroutineScope,
        private val write: suspend (ParcelFileDescriptor, CancellationSignal) -> Unit,
    ) : PrintDocumentAdapter() {
        override fun onLayout(
            oldAttributes: PrintAttributes?,
            newAttributes: PrintAttributes,
            cancellationSignal: CancellationSignal,
            callback: LayoutResultCallback,
            extras: Bundle?,
        ) {
            if (cancellationSignal.isCanceled) {
                callback.onLayoutCancelled()
                return
            }
            val info = PrintDocumentInfo.Builder("$jobName.pdf")
                .setContentType(PrintDocumentInfo.CONTENT_TYPE_DOCUMENT)
                .setPageCount(pageCount)
                .build()
            callback.onLayoutFinished(info, oldAttributes != newAttributes)
        }

        override fun onWrite(
            pages: Array<out PageRange>,
            destination: ParcelFileDescriptor,
            cancellationSignal: CancellationSignal,
            callback: WriteResultCallback,
        ) {
            scope.launch {
                val result = runCatching { write(destination, cancellationSignal) }
                when {
                    cancellationSignal.isCanceled -> callback.onWriteCancelled()
                    result.isSuccess -> callback.onWriteFinished(arrayOf(PageRange.ALL_PAGES))
                    else -> callback.onWriteFailed(result.exceptionOrNull()?.message)
                }
            }
        }
    }
}
