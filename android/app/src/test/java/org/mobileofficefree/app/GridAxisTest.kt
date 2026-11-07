package org.mobileofficefree.app

import org.junit.Assert.assertEquals
import org.junit.Test

class GridAxisTest {
    // Columns A–D: 48, 100, hidden, 20 points; then 48 each.
    private val axis = GridAxis(floatArrayOf(48f, 100f, 0f, 20f), 48f)

    @Test
    fun positionsFollowTheSizes() {
        assertEquals(0f, axis.start(0))
        assertEquals(148f, axis.start(2))
        assertEquals(148f, axis.start(3))
        assertEquals(168f, axis.start(4))
        assertEquals(168f + 96f, axis.start(6))
        assertEquals(48f, axis.size(10))
    }

    @Test
    fun findsTheLineAtAPosition() {
        assertEquals(0, axis.at(10f))
        assertEquals(1, axis.at(60f))
        assertEquals(3, axis.at(150f))
        assertEquals(4, axis.at(170f))
        assertEquals(6, axis.at(168f + 96f + 1f))
    }

    @Test
    fun oneLineCanBeResized() {
        val wider = axis.with(1, 150f)
        assertEquals(150f, wider.size(1))
        assertEquals(48f + 150f, wider.start(2))
        // Past the sized lines: the lines between take the default size.
        val far = axis.with(6, 10f)
        assertEquals(48f, far.size(5))
        assertEquals(10f, far.size(6))
        assertEquals(168f + 48f * 2 + 10f, far.start(7))
        // The original is unchanged.
        assertEquals(100f, axis.size(1))
    }
}
