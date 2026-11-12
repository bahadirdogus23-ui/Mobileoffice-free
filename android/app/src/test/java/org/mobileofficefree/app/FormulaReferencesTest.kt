package org.mobileofficefree.app

import org.junit.Assert.assertEquals
import org.junit.Test

class FormulaReferencesTest {
    private fun refs(formula: String) = formulaReferences(formula).map { it.toList() }

    @Test fun localRangesIncludeAbsoluteAndReversedReferences() {
        assertEquals(listOf(listOf(0, 0, 2, 1), listOf(4, 2, 4, 2)), refs("=TOPLA(\$B\$3:A1;C5)"))
    }

    @Test fun otherSheetRangesDoNotHighlightTheirEndOnThisSheet() {
        assertEquals(listOf(listOf(3, 3, 3, 3)), refs("=TOPLA(Sayfa2!A1:B3;'Yıllık Rapor'!C1:D9;D4)"))
        assertEquals(emptyList<List<Int>>(), refs("='Ali''nin sayfası'!A1:B3+A1!C2:D4"))
        assertEquals(emptyList<List<Int>>(), refs("=[Book.xlsx]Sheet1!A1:B3"))
    }

    @Test fun outOfBoundsRowsAndColumnsAreNotHighlighted() {
        assertEquals(listOf(listOf(1048575, 16383, 1048575, 16383)), refs("=TOPLA(A0;A1048577;XFE1;XFD1048576)"))
    }

    @Test fun textAndPartsOfNamesAreNotReferences() {
        assertEquals(emptyList<List<Int>>(), refs("=\"A1\"&\"B2\"&ÖzetA1+ÖA1+LOG10(2)"))
    }
}
