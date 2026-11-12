package org.mobileofficefree.app

internal enum class SheetPasteSource { NONE, INTERNAL, TEXT }

/** Native clipboard formats belong to one workbook, even when another workbook has the same text. */
internal fun sheetPasteSource(clipboard: String?, copied: String?, sameWorkbook: Boolean): SheetPasteSource = when {
    clipboard != null && copied != null && sameWorkbook && clipboard == copied -> SheetPasteSource.INTERNAL
    clipboard.isNullOrEmpty() -> SheetPasteSource.NONE
    else -> SheetPasteSource.TEXT
}
