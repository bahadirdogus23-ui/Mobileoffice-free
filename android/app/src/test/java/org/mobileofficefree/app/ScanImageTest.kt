package org.mobileofficefree.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ScanImageTest {
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
}
