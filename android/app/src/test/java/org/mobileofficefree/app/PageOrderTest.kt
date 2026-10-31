package org.mobileofficefree.app

import org.junit.Assert.assertEquals
import org.junit.Test

class PageOrderTest {
    @Test
    fun aDraggedPageLandsWhereItWasDropped() {
        val pages = listOf("a", "b", "c", "d")
        assertEquals(listOf("b", "c", "a", "d"), moved(pages, 0, 2))
        assertEquals(listOf("d", "a", "b", "c"), moved(pages, 3, 0))
        // Past either end: first or last.
        assertEquals(listOf("b", "c", "d", "a"), moved(pages, 0, 9))
        assertEquals(listOf("c", "a", "b", "d"), moved(pages, 2, -3))
        assertEquals(pages, moved(pages, 7, 0))
    }
}
