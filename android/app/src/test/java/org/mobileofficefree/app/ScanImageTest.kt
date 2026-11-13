package org.mobileofficefree.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ScanImageTest {
    @Test
    fun cropCornersCannotCrossCollapseOrLeaveTheImage() {
        val page = listOf(0.1f to 0.1f, 0.9f to 0.2f, 0.8f to 0.9f, 0.1f to 0.8f)
        assertTrue(ScanImage.validCorners(page))
        assertTrue(ScanImage.validCorners(listOf(0f to 0f, 1f to 0f, 1f to 1f, 0f to 1f)))
        val bad = listOf(
            listOf(page[0], page[2], page[1], page[3]), // Crossing edges.
            page.toMutableList().also { it[2] = 0.2f to 0.2f }, // Concave page.
            page.toMutableList().also { it[1] = it[0] },
            page.toMutableList().also { it[0] = Float.NaN to 0f },
            page.toMutableList().also { it[0] = -0.1f to 0f },
            page.reversed(),
            page.take(3),
        )
        bad.forEach { assertTrue("Invalid crop accepted: $it", !ScanImage.validCorners(it)) }
    }

    @Test
    fun straightenedSizeTakesTheLongerEdges() {
        val corners = listOf(10f to 10f, 210f to 20f, 220f to 320f, 0f to 300f)
        val (w, h) = ScanImage.outputSize(corners)
        assertEquals(221, w) // bottom edge ~221 > top edge ~200
        assertEquals(300, h)
    }

    @Test
    fun picksTheCornerUnderTheFinger() {
        val corners = listOf(0f to 0f, 100f to 0f, 100f to 100f, 0f to 100f)
        assertEquals(2, ScanImage.nearestCorner(corners, 95f, 90f, 30f))
        assertNull(ScanImage.nearestCorner(corners, 50f, 50f, 30f))
    }

    @Test
    fun documentLookWhitensShadowedPaperAndKeepsTextDark() {
        // A page whose right half is in shadow (paper 120 instead of 220), with dark text (40% of
        // the paper's brightness) on both halves.
        val w = 96
        val h = 96
        val px = IntArray(w * h) { i ->
            val x = i % w
            val paper = if (x < w / 2) 220 else 120
            val ink = (i / w) % 12 == 5 && x % 6 == 2
            val v = if (ink) paper * 2 / 5 else paper
            (0xFF shl 24) or (v shl 16) or (v shl 8) or v
        }
        ScanImage.applyLook(px, w, h, ScanLook.BLACK_WHITE)
        val value = { x: Int, y: Int -> px[y * w + x] and 0xFF }
        assertEquals(255, value(10, 0)) // lit paper
        assertEquals(255, value(80, 0)) // shadowed paper, white too
        assertEquals(0, value(14, 5)) // text in the light
        assertEquals(0, value(80, 5)) // text in the shadow
        val colors = IntArray(4) { 0xFF336699.toInt() }
        ScanImage.applyLook(colors, 2, 2, ScanLook.COLOR)
        assertTrue(colors.all { it == 0xFF336699.toInt() })
    }

    /** A [w]×[h] dark table with a light, slightly turned page (corners given in pixels) on it. */
    private fun photo(w: Int, h: Int, page: List<Pair<Float, Float>>): IntArray {
        fun inside(x: Float, y: Float): Boolean {
            // Inside a convex quadrilateral: on the same side of every edge.
            var sign = 0
            for (i in 0 until 4) {
                val (ax, ay) = page[i]
                val (bx, by) = page[(i + 1) % 4]
                val c = (bx - ax) * (y - ay) - (by - ay) * (x - ax)
                val s = if (c > 0) 1 else if (c < 0) -1 else 0
                if (s != 0) {
                    if (sign != 0 && s != sign) return false
                    sign = s
                }
            }
            return true
        }
        return IntArray(w * h) { i ->
            val x = (i % w).toFloat()
            val y = (i / w).toFloat()
            // A little noise on the table; the page has dark lines of "text".
            val v = if (inside(x, y)) (if ((i / w) % 9 == 0 && (i % w) % 3 != 0) 60 else 235) else 70 + (i * 7919 % 25)
            (0xFF shl 24) or (v shl 16) or (v shl 8) or v
        }
    }

    @Test
    fun findsAPageOnATable() {
        val w = 120
        val h = 160
        val page = listOf(20f to 15f, 100f to 25f, 95f to 145f, 12f to 135f)
        val found = ScanImage.detectPage(photo(w, h, page), w, h)!!
        for ((got, want) in found.zip(page)) {
            assertEquals(want.first / w, got.first, 0.04f)
            assertEquals(want.second / h, got.second, 0.04f)
        }
    }

    @Test
    fun noPageWhenNothingStandsOut() {
        val w = 64
        val h = 64
        val flat = IntArray(w * h) { (0xFF shl 24) or 0x808080 }
        assertNull(ScanImage.detectPage(flat, w, h))
        val allPaper = IntArray(w * h) { (0xFF shl 24) or 0xF0F0F0 }
        assertNull(ScanImage.detectPage(allPaper, w, h))
    }
}
