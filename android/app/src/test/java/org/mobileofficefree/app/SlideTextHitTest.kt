package org.mobileofficefree.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import org.mobileofficefree.engine.SlideText

class SlideTextHitTest {
    private val cell = SlideText(1u, 0f, 0f, 100f, 50f, "Cell", 0, 0)
    private val overlay = SlideText(2u, 10f, 10f, 20f, 20f, "Overlay", -1, -1)

    @Test fun editingPickedTableCannotOpenOverlappingTextBox() {
        assertEquals(cell, slideTextAt(listOf(cell, overlay), 15f, 15f, 1u))
        assertEquals(overlay, slideTextAt(listOf(cell, overlay), 15f, 15f))
    }

    @Test fun picksTheTappedCellOfTheSelectedTable() {
        val next = cell.copy(x = 100f, text = "Next", col = 1)
        assertEquals(next, slideTextAt(listOf(cell, next, overlay), 120f, 15f, 1u))
        assertNull(slideTextAt(listOf(cell, next, overlay), 500f, 15f, 1u))
        assertNull(slideTextAt(listOf(overlay), 15f, 15f, 1u))
    }
}
