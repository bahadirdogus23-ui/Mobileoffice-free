package org.mobileofficefree.app

import kotlin.math.hypot
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToInt

/** How a scanned page looks in the PDF. */
enum class ScanLook(val label: Int) {
    COLOR(R.string.scan_color),
    DOCUMENT(R.string.scan_document),
    BLACK_WHITE(R.string.scan_black_white),
}

/** Pure pixel work for document scanning (no Android types, so it is tested on the JVM). */
object ScanImage {
    /** Corners a little inside the picture: where the page starts before the person moves them. */
    val defaultCorners = listOf(0.04f to 0.04f, 0.96f to 0.04f, 0.96f to 0.96f, 0.04f to 0.96f)

    /**
     * The straightened page's size in pixels for `corners` (top-left, top-right, bottom-right,
     * bottom-left, in pixels): the longer of each pair of opposite edges.
     */
    fun outputSize(corners: List<Pair<Float, Float>>): Pair<Int, Int> {
        fun len(a: Pair<Float, Float>, b: Pair<Float, Float>) = hypot(a.first - b.first, a.second - b.second)
        val (tl, tr, br, bl) = corners
        val w = max(len(tl, tr), len(bl, br)).roundToInt().coerceAtLeast(1)
        val h = max(len(tl, bl), len(tr, br)).roundToInt().coerceAtLeast(1)
        return w to h
    }

    /** The corner nearest to (x, y), if within `reach`. */
    fun nearestCorner(corners: List<Pair<Float, Float>>, x: Float, y: Float, reach: Float): Int? =
        corners.indices.minByOrNull { hypot(corners[it].first - x, corners[it].second - y) }
            ?.takeIf { hypot(corners[it].first - x, corners[it].second - y) <= reach }

    /**
     * Where the paper is in a photo ([argb], [w]×[h], small is enough): the largest bright
     * region (paper is lighter than the table under it), its corners taken as the region's
     * extreme points along the diagonals. Top-left, top-right, bottom-right, bottom-left as
     * fractions of the picture; null when no page stands out.
     */
    fun detectPage(argb: IntArray, w: Int, h: Int): List<Pair<Float, Float>>? {
        if (w < 8 || h < 8 || argb.size < w * h) return null
        val gray = IntArray(w * h) { i ->
            val c = argb[i]
            ((c shr 16 and 0xFF) * 299 + (c shr 8 and 0xFF) * 587 + (c and 0xFF) * 114) / 1000
        }
        val threshold = otsu(gray)
        // Paper must be clearly lighter than what is around it.
        val bright = BooleanArray(gray.size) { gray[it] > threshold }
        // The largest 4-connected bright region.
        val label = IntArray(gray.size)
        var best = -1
        var bestSize = 0
        var next = 0
        val queue = IntArray(gray.size)
        for (start in gray.indices) {
            if (!bright[start] || label[start] != 0) continue
            next++
            var head = 0
            var tail = 0
            queue[tail++] = start
            label[start] = next
            while (head < tail) {
                val i = queue[head++]
                val x = i % w
                val y = i / w
                fun visit(j: Int) {
                    if (bright[j] && label[j] == 0) {
                        label[j] = next
                        queue[tail++] = j
                    }
                }
                if (x > 0) visit(i - 1)
                if (x < w - 1) visit(i + 1)
                if (y > 0) visit(i - w)
                if (y < h - 1) visit(i + w)
            }
            if (tail > bestSize) {
                bestSize = tail
                best = next
            }
        }
        // Too small to be the page, or everything (no edge to find).
        if (best < 0 || bestSize < gray.size / 10 || bestSize > gray.size * 97 / 100) return null
        var tl = 0
        var tr = 0
        var br = 0
        var bl = 0
        var minSum = Int.MAX_VALUE
        var maxSum = Int.MIN_VALUE
        var minDiff = Int.MAX_VALUE
        var maxDiff = Int.MIN_VALUE
        for (i in label.indices) {
            if (label[i] != best) continue
            val x = i % w
            val y = i / w
            if (x + y < minSum) { minSum = x + y; tl = i }
            if (x + y > maxSum) { maxSum = x + y; br = i }
            if (x - y > maxDiff) { maxDiff = x - y; tr = i }
            if (x - y < minDiff) { minDiff = x - y; bl = i }
        }
        val corners = listOf(tl, tr, br, bl).map { i -> (i % w + 0.5f) / w to (i / w + 0.5f) / h }
        // A sliver is not a page: each side at least a fifth of the picture's.
        val (pw, ph) = outputSize(corners.map { (x, y) -> x * 1000 to y * 1000 })
        if (pw < 200 || ph < 200) return null
        return corners
    }

    /** Otsu's threshold: the grey level that best splits the picture into dark and light. */
    private fun otsu(gray: IntArray): Int {
        val hist = IntArray(256)
        for (v in gray) hist[v.coerceIn(0, 255)]++
        val total = gray.size.toDouble()
        var sum = 0.0
        for (t in 0..255) sum += t * hist[t].toDouble()
        var sumB = 0.0
        var wB = 0.0
        var best = 0.0
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
     * [argb] made to look like a scanned page: grey, the paper (estimated as the brightest
     * nearby tone) made white, so shadows and uneven light go; black and white also thresholds.
     */
    fun applyLook(argb: IntArray, w: Int, h: Int, look: ScanLook) {
        if (look == ScanLook.COLOR) return
        val gray = IntArray(argb.size) { i ->
            val c = argb[i]
            ((c shr 16 and 0xFF) * 299 + (c shr 8 and 0xFF) * 587 + (c and 0xFF) * 114) / 1000
        }
        val paper = paperLevel(gray, w, h)
        for (i in gray.indices) {
            val bg = paper[i].coerceAtLeast(1)
            // Normalise to the paper, then push the darker half darker for crisp text.
            var v = (gray[i] * 255 / bg).coerceAtMost(255)
            v = if (v >= 230) 255 else ((v - 40) * 255 / 190).coerceIn(0, 255)
            if (look == ScanLook.BLACK_WHITE) v = if (v >= 150) 255 else 0
            argb[i] = (0xFF shl 24) or (v shl 16) or (v shl 8) or v
        }
    }

    /**
     * The paper's brightness around each pixel: the brightest value in blocks of the picture
     * (text is dark and thin, the paper around it bright), smoothed between blocks.
     */
    private fun paperLevel(gray: IntArray, w: Int, h: Int): IntArray {
        val block = max(8, min(w, h) / 24)
        val bw = (w + block - 1) / block
        val bh = (h + block - 1) / block
        val top = IntArray(bw * bh)
        for (y in 0 until h) {
            val row = (y / block) * bw
            for (x in 0 until w) {
                val k = row + x / block
                val v = gray[y * w + x]
                if (v > top[k]) top[k] = v
            }
        }
        // Each pixel takes its block's level, blended with its neighbours' (bilinear).
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
                val a = top[y0 * bw + x0] * (1 - tx) + top[y0 * bw + x1] * tx
                val b = top[y1 * bw + x0] * (1 - tx) + top[y1 * bw + x1] * tx
                out[y * w + x] = (a * (1 - ty) + b * ty).roundToInt()
            }
        }
        return out
    }
}
