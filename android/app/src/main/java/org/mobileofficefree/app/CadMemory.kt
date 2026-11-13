package org.mobileofficefree.app

import java.io.File
import java.util.Locale

/** How the CAD engine reads a file, which sets how much memory opening it takes. */
internal enum class CadFormat(val label: String) {
    DWG("DWG"),
    DXF("DXF"),
    BINARY_DXF("ikili DXF"),
}

/**
 * The memory guard before a drawing is opened. The pinned engine needs many times a file's size
 * while it opens it (DWG is first turned into a whole DXF text, and DXF is held as one list of
 * all its group codes: docs/upstream-notlar/cadcraft-performans.md), all of it native memory the
 * Java heap limit does not cover. When that peak would not fit in the phone's free memory,
 * Android ends the app mid-open, so the person is asked first.
 */
internal object CadMemory {
    /**
     * Peak resident memory (VmHWM) while opening, per byte of file. Measured on the pinned engine
     * (release build, synthetic plans of 32k–612k objects written by the engine itself):
     * DWG 1.0–18.2 MB → 185 MB–3.5 GB (194–201×), ASCII DXF 6–118 MB → 23×,
     * binary DXF 4.5–71 MB → 31×. Linear in the file size; the largest measured factor is used.
     */
    const val DWG_FACTOR = 200.0
    const val DXF_FACTOR = 24.0
    const val BINARY_DXF_FACTOR = 32.0

    /** What opening any drawing takes on top of the file's own share (engine tables, fonts). */
    const val BASE_BYTES = 32L shl 20

    /** Kept free beyond the system's own low-memory line, for the screen and the other apps. */
    const val MIN_MARGIN_BYTES = 256L shl 20

    private val BINARY_SENTINEL = "AutoCAD Binary DXF\r\n".toByteArray(Charsets.US_ASCII)

    /** The format the engine will read [bytes] as (the same test the engine makes). */
    fun format(bytes: ByteArray): CadFormat = when {
        isDwg(bytes) -> CadFormat.DWG
        startsWith(bytes, BINARY_SENTINEL) -> CadFormat.BINARY_DXF
        else -> CadFormat.DXF
    }

    /** About the most memory opening a [format] file of [size] bytes takes. */
    fun estimate(format: CadFormat, size: Long): Long {
        val factor = when (format) {
            CadFormat.DWG -> DWG_FACTOR
            CadFormat.DXF -> DXF_FACTOR
            CadFormat.BINARY_DXF -> BINARY_DXF_FACTOR
        }
        return BASE_BYTES + (size.coerceAtLeast(0) * factor).toLong()
    }

    /**
     * What may be spent opening the drawing: the free memory Android reports less a margin, at
     * least its own low-memory line ([threshold]), below which it starts ending apps.
     */
    fun budget(availMem: Long, threshold: Long): Long = availMem - maxOf(threshold, MIN_MARGIN_BYTES)

    /** Whether the person should be asked before opening. */
    fun tooLarge(estimate: Long, budget: Long): Boolean = estimate > budget

    /** "850 MB", "3.4 GB" (for the dialog in the person's [locale], for the report in [Locale.ROOT]). */
    fun label(bytes: Long, locale: Locale): String {
        val mb = bytes.coerceAtLeast(0) / (1024.0 * 1024.0)
        return if (mb < 1000) String.format(locale, "%.0f MB", mb) else String.format(locale, "%.1f GB", mb / 1024.0)
    }

    /** The process's peak resident memory so far (VmHWM) in bytes, or 0 when it cannot be read. */
    fun peakResident(): Long = runCatching {
        File("/proc/self/status").useLines { lines ->
            lines.firstOrNull { it.startsWith("VmHWM:") }?.split(Regex("\\s+"))?.getOrNull(1)?.toLongOrNull()?.times(1024L)
        } ?: 0L
    }.getOrDefault(0L)

    /** "AC10" and two digits ("AC1032"), as the engine's `is_dwg`. */
    private fun isDwg(b: ByteArray): Boolean =
        b.size > 6 && startsWith(b, DWG_MAGIC) && b[4].toInt() in DIGITS && b[5].toInt() in DIGITS

    private val DWG_MAGIC = "AC10".toByteArray(Charsets.US_ASCII)
    private val DIGITS = '0'.code..'9'.code

    private fun startsWith(b: ByteArray, prefix: ByteArray): Boolean =
        b.size >= prefix.size && prefix.indices.all { b[it] == prefix[it] }
}
