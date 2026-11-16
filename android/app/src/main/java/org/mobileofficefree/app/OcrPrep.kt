package org.mobileofficefree.app

import kotlin.math.max
import kotlin.math.min

/**
 * The picture made ready for Tesseract (pure Kotlin on ARGB pixels, so it is unit tested): a
 * size where letters are a comfortable height, grey, its contrast stretched, then black text on
 * white. Photos of a sign or a page with uneven light and a textured wall around them are read
 * far better this way than as they come.
 */
object OcrImage {
    /** The longest side Tesseract gets: big letters stay sharp, small ones stay readable, and it is quick. */
    const val TARGET_LONG_SIDE = 2600

    /** Pictures whose longest side lies in this range are read at the size they are. */
    private const val MIN_LONG_SIDE = 2000
    private const val MAX_LONG_SIDE = 3000

    /** How much a small picture is enlarged at most (more only blurs). */
    private const val MAX_UPSCALE = 2.5

    /**
     * The factor a `w`×`h` picture is scaled by before reading: 1 when its longest side is
     * already 2000–3000 px, else what brings it to [TARGET_LONG_SIDE] (enlarging at most 2.5×).
     */
    fun scaleFactor(w: Int, h: Int): Double {
        val long = max(w, h)
        if (long <= 0) return 1.0
        if (long in MIN_LONG_SIDE..MAX_LONG_SIDE) return 1.0
        return min(TARGET_LONG_SIDE.toDouble() / long, MAX_UPSCALE)
    }

    /** Each ARGB pixel's brightness, 0 (black) to 255 (white). */
    fun gray(argb: IntArray): IntArray = IntArray(argb.size) { i ->
        val c = argb[i]
        ((c shr 16 and 0xFF) * 299 + (c shr 8 and 0xFF) * 587 + (c and 0xFF) * 114) / 1000
    }

    /**
     * [gray] stretched in place so its darkest 1 % become black and its brightest 1 % white: a
     * grey, faded photo gets the full range. A flat picture is left as it is.
     */
    fun stretchContrast(gray: IntArray) {
        if (gray.isEmpty()) return
        val hist = histogram(gray)
        val cut = gray.size / 100
        var lo = 0
        var seen = 0
        while (lo < 255 && seen + hist[lo] <= cut) seen += hist[lo++]
        var hi = 255
        seen = 0
        while (hi > 0 && seen + hist[hi] <= cut) seen += hist[hi--]
        if (hi - lo < 16) return
        val span = hi - lo
        for (i in gray.indices) gray[i] = ((gray[i] - lo) * 255 / span).coerceIn(0, 255)
    }

    /** Otsu's threshold: the grey level that best splits [gray] into dark and light (dark is `<=`). */
    fun otsu(gray: IntArray): Int {
        val hist = histogram(gray)
        val total = gray.size.toDouble()
        var sum = 0.0
        for (t in 0..255) sum += t * hist[t].toDouble()
        var sumB = 0.0
        var wB = 0.0
        var best = -1.0
        var threshold = 127
        for (t in 0..255) {
            wB += hist[t]
            if (wB == 0.0) continue
            val wF = total - wB
            if (wF == 0.0) break
            sumB += t * hist[t].toDouble()
            val mB = sumB / wB
            val mF = (sum - sumB) / wF
            val between = wB * wF * (mB - mF) * (mB - mF)
            if (between > best) {
                best = between
                threshold = t
            }
        }
        return threshold
    }

    /**
     * Black (0) and white (255) from [gray] (`w`×`h`): a pixel is ink when it is darker than
     * Otsu's split of the whole picture *and* clearly darker (by 15 %) than its surroundings (the
     * mean of a window about a sixteenth of the picture across). The first keeps a lit wall or
     * a shadowed margin from turning into ink; the second follows uneven light across a photo,
     * and the window is wide enough that thick letters stay solid.
     */
    fun binarize(gray: IntArray, w: Int, h: Int): IntArray {
        require(gray.size == w * h) { "size" }
        if (gray.isEmpty()) return IntArray(0)
        val global = otsu(gray)
        val mean = localMean(gray, w, h, max(16, max(w, h) / 16))
        return IntArray(gray.size) { i ->
            val v = gray[i]
            if (v <= global && v * 100 < mean[i] * 85) 0 else 255
        }
    }

    /**
     * The whole preparation of `w`×`h` ARGB pixels (already scaled with [scaleFactor]): grey,
     * stretched, black on white; returned as opaque ARGB pixels of the same size.
     */
    fun prepare(argb: IntArray, w: Int, h: Int): IntArray {
        val g = gray(argb)
        stretchContrast(g)
        val bw = binarize(g, w, h)
        return IntArray(bw.size) { i ->
            val v = bw[i]
            (0xFF shl 24) or (v shl 16) or (v shl 8) or v
        }
    }

    private fun histogram(gray: IntArray): IntArray {
        val hist = IntArray(256)
        for (v in gray) hist[v.coerceIn(0, 255)]++
        return hist
    }

    /**
     * The mean brightness around each pixel: the means of `block`-sized squares, blended between
     * square centres (bilinear), so only the small grid of squares is kept besides the result.
     */
    private fun localMean(gray: IntArray, w: Int, h: Int, block: Int): IntArray {
        val bw = (w + block - 1) / block
        val bh = (h + block - 1) / block
        val sum = LongArray(bw * bh)
        val count = IntArray(bw * bh)
        for (y in 0 until h) {
            val row = (y / block) * bw
            for (x in 0 until w) {
                val k = row + x / block
                sum[k] += gray[y * w + x].toLong()
                count[k]++
            }
        }
        val m = FloatArray(bw * bh) { k -> if (count[k] == 0) 255f else sum[k].toFloat() / count[k] }
        val out = IntArray(gray.size)
        for (y in 0 until h) {
            val fy = ((y + 0.5f) / block - 0.5f).coerceIn(0f, (bh - 1).toFloat())
            val y0 = fy.toInt()
            val y1 = min(y0 + 1, bh - 1)
            val ty = fy - y0
            for (x in 0 until w) {
                val fx = ((x + 0.5f) / block - 0.5f).coerceIn(0f, (bw - 1).toFloat())
                val x0 = fx.toInt()
                val x1 = min(x0 + 1, bw - 1)
                val tx = fx - x0
                val top = m[y0 * bw + x0] * (1 - tx) + m[y0 * bw + x1] * tx
                val bottom = m[y1 * bw + x0] * (1 - tx) + m[y1 * bw + x1] * tx
                out[y * w + x] = (top * (1 - ty) + bottom * ty).toInt()
            }
        }
        return out
    }
}

/** A word Tesseract read, with its confidence (0–100) and, where needed, its box in pixels. */
data class OcrReadWord(
    val text: String,
    val confidence: Float,
    val left: Int = 0,
    val top: Int = 0,
    val right: Int = 0,
    val bottom: Int = 0,
)

/** A line of words in reading order; [newBlock] when it starts a new paragraph or block. */
data class OcrReadLine(val words: List<OcrReadWord>, val newBlock: Boolean = false)

/**
 * Which recognised lines are text and which are noise. Around a photographed sign, Tesseract
 * reads the wall's texture, a frame or a shadow as "lines" like `i, .` or `RAMA a ON`: few
 * letters, many marks, low confidence. Those are left out; real lines keep their order.
 */
object OcrLines {
    /** A line whose words Tesseract is on average less sure of than this (0–100) is noise. */
    const val MIN_CONFIDENCE = 40f

    /** Below this confidence a line of mostly one- and two-letter fragments is noise too. */
    private const val FRAGMENT_CONFIDENCE = 60f

    /** Whether [line] is text worth keeping. */
    fun keep(line: OcrReadLine): Boolean {
        val words = line.words.filter { it.text.isNotBlank() }
        if (words.isEmpty()) return false
        var marks = 0
        var alnum = 0
        var weighted = 0.0
        for (w in words) {
            val n = w.text.count { it.isLetterOrDigit() }
            marks += w.text.count { !it.isWhitespace() }
            alnum += n
            // Longer words weigh more: one sure long word outweighs a doubtful stray letter.
            weighted += w.confidence.toDouble() * max(n, 1)
        }
        if (alnum < 2) return false
        // Mostly punctuation and marks: a texture, not words.
        if (alnum * 10 < marks * 6) return false
        val confidence = weighted / words.sumOf { max(it.text.count { c -> c.isLetterOrDigit() }, 1) }
        if (confidence < MIN_CONFIDENCE) return false
        // Fragments ("a ON", "i I") that Tesseract is unsure of.
        val meanLength = alnum.toDouble() / words.size
        if (confidence < FRAGMENT_CONFIDENCE && meanLength < 2.5) return false
        return true
    }

    /** The lines worth keeping, in order. */
    fun kept(lines: List<OcrReadLine>): List<OcrReadLine> = lines.filter(::keep)

    /**
     * The text of the kept [lines]: words joined by spaces, lines by line breaks, and a blank
     * line where a new paragraph or block starts (as Tesseract's own text has).
     */
    fun text(lines: List<OcrReadLine>): String {
        val out = StringBuilder()
        // A block whose first line was noise still starts where its next real line does.
        var blockStarts = false
        for (line in lines) {
            blockStarts = blockStarts || line.newBlock
            if (!keep(line)) continue
            if (out.isNotEmpty()) out.append(if (blockStarts) "\n\n" else "\n")
            out.append(line.words.filter { it.text.isNotBlank() }.joinToString(" ") { it.text.trim() })
            blockStarts = false
        }
        return out.toString()
    }
}
