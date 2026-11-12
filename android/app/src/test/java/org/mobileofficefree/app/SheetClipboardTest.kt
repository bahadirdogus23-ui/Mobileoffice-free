package org.mobileofficefree.app

import org.junit.Assert.assertEquals
import org.junit.Test

class SheetClipboardTest {
    @Test fun sameWorkbookKeepsFormulaAndFormatClipboard() {
        assertEquals(SheetPasteSource.INTERNAL, sheetPasteSource("12\tŞule", "12\tŞule", true))
    }

    @Test fun anotherWorkbookMustReceiveTextEvenWhenTheContentsMatch() {
        assertEquals(SheetPasteSource.TEXT, sheetPasteSource("12\tŞule", "12\tŞule", false))
        assertEquals(SheetPasteSource.TEXT, sheetPasteSource("12\tŞule", null, false))
    }

    @Test fun anExternalCopyOverridesTheNativeClipboard() {
        assertEquals(SheetPasteSource.TEXT, sheetPasteSource("Yeni", "Eski", true))
    }

    @Test fun clearingThePhoneClipboardDoesNotPasteAnOlderNativeCopy() {
        assertEquals(SheetPasteSource.NONE, sheetPasteSource(null, "Eski", true))
        assertEquals(SheetPasteSource.NONE, sheetPasteSource("", "Eski", true))
        assertEquals(SheetPasteSource.NONE, sheetPasteSource("", null, false))
        // An explicitly copied empty string remains a valid native copy.
        assertEquals(SheetPasteSource.INTERNAL, sheetPasteSource("", "", true))
    }
}
