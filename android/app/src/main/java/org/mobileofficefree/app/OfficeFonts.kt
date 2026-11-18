package org.mobileofficefree.app

import android.content.Context
import org.mobileofficefree.engine.registerPdfFonts
import org.mobileofficefree.engine.registerSlideFonts
import org.mobileofficefree.engine.registerWordFonts

/**
 * The metric-compatible stand-ins for Office's fonts that the app ships as assets
 * (`assets/fonts/`: Carlito for Calibri, Caladea for Cambria, Liberation Sans for Arial,
 * Liberation Serif for Times New Roman; SIL OFL 1.1, unmodified). Each engine gets them once,
 * before its first document opens, so text breaks into the same lines as in PowerPoint and Word.
 * Without them the engines fall back to the fonts compiled into the library (Inter, Source Sans 3…).
 *
 * Called on a document's engine thread, never on the UI thread: the files are ~6 MB to read.
 * The engines keep their own copy, so the bytes read here are dropped right after.
 */
object OfficeFonts {
    private const val DIR = "fonts"

    private var slides = false
    private var word = false
    private var pdf = false

    /** Hands the fonts to the slide engine, the first time only. */
    @Synchronized
    fun forSlides(context: Context) {
        if (slides) return
        slides = true
        val files = read(context) ?: return
        Measurements.time("Sunu: yazı tiplerini yükleme") { registerSlideFonts(files) }
    }

    /** Hands the fonts to the word-processing engine, the first time only. */
    @Synchronized
    fun forWord(context: Context) {
        if (word) return
        word = true
        val files = read(context) ?: return
        Measurements.time("Word: yazı tiplerini yükleme") { registerWordFonts(files) }
    }

    /**
     * Hands Liberation Sans and Serif to the PDF engine, the first time only: it embeds them in the
     * copy shown on screen for fonts a PDF names without embedding whose letters Android's renderer
     * lacks (Ğ ğ İ Ş ş in old Turkish PDFs' Helvetica). Only called for such a file.
     */
    @Synchronized
    fun forPdf(context: Context) {
        if (pdf) return
        pdf = true
        val files = read(context) { it.startsWith("Liberation") } ?: return
        Measurements.time("PDF: yazı tiplerini yükleme") { registerPdfFonts(files) }
    }

    /**
     * The `.ttf` assets, in name order; null when they cannot be read (the engines then keep their
     * compiled-in fonts: a document still opens).
     */
    private fun read(context: Context, wanted: (String) -> Boolean = { true }): List<ByteArray>? = runCatching {
        val assets = context.applicationContext.assets
        assets.list(DIR).orEmpty().filter { it.endsWith(".ttf") && wanted(it) }.sorted().map { name ->
            assets.open("$DIR/$name").use { it.readBytes() }
        }
    }.onFailure { Diagnostics.event("Yazı tipleri okunamadı: ${it.message}") }.getOrNull()
}
