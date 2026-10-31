package org.mobileofficefree.app

/**
 * Positions along one axis of a sheet (columns or rows): the given sizes for the first lines,
 * [default] for every line after them. Sizes are in points; 0 is a hidden line.
 */
class GridAxis(private val sizes: FloatArray, private val default: Float) {
    /** start[i] = where line i starts; start[n] = the end of the sized lines. */
    private val start = FloatArray(sizes.size + 1).also { s ->
        for (i in sizes.indices) s[i + 1] = s[i] + sizes[i]
    }

    fun size(i: Int): Float = if (i < sizes.size) sizes[i] else default

    fun start(i: Int): Float = if (i <= sizes.size) start[i] else start[sizes.size] + (i - sizes.size) * default

    /** The line at position [pos] (≥ 0). */
    fun at(pos: Float): Int {
        val end = start[sizes.size]
        if (pos >= end) return sizes.size + ((pos - end) / default.coerceAtLeast(0.01f)).toInt()
        // The last line starting at or before pos, skipping hidden (zero-size) lines.
        var lo = 0
        var hi = sizes.size - 1
        while (lo < hi) {
            val mid = (lo + hi + 1) / 2
            if (start[mid] <= pos) lo = mid else hi = mid - 1
        }
        while (lo < sizes.size - 1 && sizes[lo] == 0f) lo++
        return lo
    }
}
