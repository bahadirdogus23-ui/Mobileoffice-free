package org.mobileofficefree.app

import android.content.Context
import android.graphics.Rect
import android.graphics.pdf.PdfDocument
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

    fun print(context: Context, vm: PagedViewModel, state: PagedUiState) {
        val manager = context.getSystemService(PrintManager::class.java) ?: return
        val jobName = state.fileName.substringBeforeLast('.')
        manager.print(jobName, Adapter(context, vm, state, jobName), null)
    }

    private class Adapter(
        private val context: Context,
        private val vm: PagedViewModel,
        private val state: PagedUiState,
        private val jobName: String,
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
                .setPageCount(state.pages.size)
                .build()
            callback.onLayoutFinished(info, oldAttributes != newAttributes)
        }

        override fun onWrite(
            pages: Array<out PageRange>,
            destination: ParcelFileDescriptor,
            cancellationSignal: CancellationSignal,
            callback: WriteResultCallback,
        ) {
            vm.viewModelScope.launch {
                val result = runCatching {
                    if (vm.printsOriginal) copyOriginal(destination) else drawPages(destination, cancellationSignal)
                }
                when {
                    cancellationSignal.isCanceled -> callback.onWriteCancelled()
                    result.isSuccess -> callback.onWriteFinished(arrayOf(PageRange.ALL_PAGES))
                    else -> callback.onWriteFailed(result.exceptionOrNull()?.message)
                }
            }
        }

        private suspend fun copyOriginal(destination: ParcelFileDescriptor) = withContext(Dispatchers.IO) {
            val input = context.contentResolver.openInputStream(state.uri) ?: error("no data")
            input.use { src -> FileOutputStream(destination.fileDescriptor).use { src.copyTo(it) } }
        }

        private suspend fun drawPages(destination: ParcelFileDescriptor, cancel: CancellationSignal) {
            val pdf = PdfDocument()
            try {
                for ((i, size) in state.pages.withIndex()) {
                    if (cancel.isCanceled) return
                    val (w, h) = size
                    val bmp = vm.renderForPrint(i, (w * PIXELS_PER_POINT).toInt()) ?: error("page ${i + 1}")
                    val page = pdf.startPage(PdfDocument.PageInfo.Builder(w.toInt(), h.toInt(), i + 1).create())
                    page.canvas.drawBitmap(bmp, null, Rect(0, 0, w.toInt(), h.toInt()), null)
                    pdf.finishPage(page)
                    bmp.recycle()
                }
                withContext(Dispatchers.IO) { FileOutputStream(destination.fileDescriptor).use { pdf.writeTo(it) } }
            } finally {
                pdf.close()
            }
        }
    }
}
