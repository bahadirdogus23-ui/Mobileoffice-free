package org.mobileofficefree.app

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Test

class SelectionBoundsTest {
    // A1:D1 merged (a title), B3:C4 merged.
    private val merges = listOf(intArrayOf(0, 0, 0, 3), intArrayOf(2, 1, 3, 2))
    private fun state(r: Int, c: Int, er: Int = r, ec: Int = c) =
        SheetUiState("a.xlsx", listOf("Sayfa1"), 0, 10, 10, selectedRow = r, selectedCol = c, endRow = er, endCol = ec, merges = merges)

    @Test
    fun aMergedCellFormatsItsWholeArea() {
        assertEquals("A1:D1", formatRange(state(0, 0)))
        assertEquals("B3:C4", formatRange(state(2, 1)))
        // The name box still shows the cell itself, as Excel's does.
        assertEquals("A1", rangeName(state(0, 0)))
    }

    @Test
    fun aRangeTakesInTheMergesItTouches() {
        // A2:B3 touches B3:C4, which grows it to A2:C4.
        assertArrayEquals(intArrayOf(1, 0, 3, 2), selectionBounds(state(1, 0, 2, 1)))
        assertEquals("E5", formatRange(state(4, 4)))
    }

    @Test
    fun rowAndColumnOperationsUseTheEntireReversedSelection() {
        val selected = state(6, 4, 4, 2)
        assertEquals("5:7", selectedLineRange(selected, true))
        assertEquals("C:E", selectedLineRange(selected, false))
    }

    @Test
    fun headerSelectionIsNotWidenedByAMergedTitle() {
        val column = state(0, 1, 9, 1).copy(wholeLine = true)
        assertEquals("B:B", selectedLineRange(column, false))
        assertEquals("1:10", selectedLineRange(column, true))
    }

    @Test
    fun operationsOnAMergedCellCoverItsRowsAndColumns() {
        val merged = state(2, 1)
        assertEquals("3:4", selectedLineRange(merged, true))
        assertEquals("B:C", selectedLineRange(merged, false))
    }
}
