package org.mobileofficefree.app

import android.app.Application
import android.content.ContentResolver
import android.net.Uri
import android.os.SystemClock
import android.provider.OpenableColumns
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.util.Locale
import org.mobileofficefree.engine.CellLook
import org.mobileofficefree.engine.SheetDocument
import org.mobileofficefree.engine.nativeVersion

/** Everything the spreadsheet screen shows, apart from the cell texts (see [SheetViewModel.cellText]). */
/** The "PDF" sheet's choices; margins are GridCraft's presets (normal, wide, narrow). */
data class PdfSetup(
    val allSheets: Boolean = true,
    val range: String = "",
    val landscape: Boolean = false,
    val paper: String = "A4",
    val margins: String = "normal",
    val fitToPage: Boolean = false,
    val gridlines: Boolean = false,
)

data class SheetUiState(
    val fileName: String,
    val sheetNames: List<String>,
    val sheet: Int,
    /** Used area of the current sheet, counted from A1. */
    val usedRows: Int,
    val usedCols: Int,
    val selectedRow: Int = 0,
    val selectedCol: Int = 0,
    /** What the formula bar shows for the selected cell. */
    val editText: String = "",
    /** Column widths and row heights (points) as the file has them. */
    val columns: GridAxis = GridAxis(FloatArray(0), 48f),
    val rows: GridAxis = GridAxis(FloatArray(0), 15f),
    /** Rows at the top and columns at the left that stay in view (Excel's frozen panes). */
    val frozenRows: Int = 0,
    val frozenCols: Int = 0,
    /** Merged ranges: [first row, first column, last row, last column]. */
    val merges: List<IntArray> = emptyList(),
)

sealed interface Screen {
    data object Home : Screen
    data object Loading : Screen
    data class Sheet(val state: SheetUiState) : Screen
}

class SheetViewModel(app: Application) : AndroidViewModel(app) {

    /** Engines are not thread-safe per document: every engine call runs on this one thread. */
    @OptIn(ExperimentalCoroutinesApi::class)
    private val engine: CoroutineDispatcher = Dispatchers.IO.limitedParallelism(1)

    private var doc: SheetDocument? = null

    var screen by mutableStateOf<Screen>(Screen.Home)
        private set

    /** A one-shot message for the snackbar (null when there is none). */
    var message by mutableStateOf<String?>(null)
        private set

    /** Import warnings to show once after opening. */
    var importNotes by mutableStateOf<List<String>>(emptyList())
        private set

    /** Cell texts, fetched from the engine in blocks of [BLOCK_ROWS]×[BLOCK_COLS] as the grid scrolls. */
    private val blocks = mutableStateMapOf<Long, List<String>>()
    /** How the cells of each block look (bold, colours, alignment), fetched with the texts. */
    private val looks = mutableStateMapOf<Long, List<CellLook>>()
    private val pending = HashSet<Long>()
    /** Where each loaded block is, so that an edit can fetch them all again. */
    private val blockAt = HashMap<Long, Triple<Int, Int, Int>>()

    /** Formulas are typed the Turkish way; function names offered while typing one. */
    var turkish by mutableStateOf(false)
        private set
    var functionNames by mutableStateOf<List<String>>(emptyList())
        private set

    /** The opened file, saved over by "Save" when it is an XLSX. */
    private var source: Uri? = null
    var canSaveInPlace by mutableStateOf(false)
        private set
    var hasUnsaved by mutableStateOf(false)
        private set

    val engineVersion: String by lazy { runCatching { nativeVersion() }.getOrDefault("?") }

    fun consumeMessage() {
        message = null
    }

    fun dismissImportNotes() {
        importNotes = emptyList()
    }

    /** Edits since the workbook was opened or last saved; closing asks first while true. */
    private var unsaved = false
        set(value) {
            field = value
            hasUnsaved = value
        }
    private var closeAfterSave = false
    var askClose by mutableStateOf(false)
        private set

    /** Back or the arrow: closes, asking first when there are unsaved edits. */
    fun requestClose() {
        if (unsaved) askClose = true else close()
    }

    /** The answer to "save the changes?": [saveFirst] true means a "Save as" is starting, false discards, null stays. */
    fun answerClose(saveFirst: Boolean?) {
        askClose = false
        when (saveFirst) {
            true -> closeAfterSave = true
            false -> close()
            null -> Unit
        }
    }

    /** "Save as" was cancelled: stay open. */
    fun saveCancelled() {
        closeAfterSave = false
    }

    fun open(uri: Uri) {
        unsaved = false
        val resolver = getApplication<Application>().contentResolver
        screen = Screen.Loading
        val started = SystemClock.elapsedRealtime()
        viewModelScope.launch {
            val result = withContext(engine) {
                runCatching {
                    val info = FileInfo.of(resolver, uri)
                    // The engine picks its reader (and later its writer) from the extension.
                    val name = info.typedName
                    val bytes = resolver.openInputStream(uri)?.use { it.readBytes() }
                        ?: error("no data")
                    val state = replaceDocument(SheetDocument.open(name, bytes, null))
                    source = uri
                    canSaveInPlace = name.endsWith(".xlsx", ignoreCase = true)
                    Diagnostics.record("SHEET ${name.substringAfterLast('.')} açma (${sizeLabel(info.size)})", started)
                    RecentFiles(getApplication()).remember(uri, info.name, DocumentKind.SHEET, info.size)
                    state to name
                }
            }
            result.fold(
                onSuccess = { (state, name) ->
                    screen = Screen.Sheet(state.copy(fileName = name))
                    importNotes = withContext(engine) { doc?.warnings().orEmpty() }
                },
                onFailure = {
                    screen = Screen.Home
                    message = text(R.string.open_failed, it.message ?: it.javaClass.simpleName)
                },
            )
        }
    }

    fun newBlank() {
        unsaved = false
        screen = Screen.Loading
        val started = SystemClock.elapsedRealtime()
        viewModelScope.launch {
            val name = text(R.string.new_workbook_name)
            source = null
            canSaveInPlace = false
            screen = Screen.Sheet(withContext(engine) { replaceDocument(SheetDocument.blank()) }.copy(fileName = name))
        }
    }

    fun close() {
        unsaved = false
        closeAfterSave = false
        val old = doc
        doc = null
        blocks.clear()
        looks.clear()
        blockAt.clear()
        screen = Screen.Home
        viewModelScope.launch(engine) { old?.close() }
    }

    fun selectSheet(index: Int) {
        val current = (screen as? Screen.Sheet)?.state ?: return
        viewModelScope.launch {
            val state = withContext(engine) {
                val d = doc ?: return@withContext null
                d.setActiveSheet(index.toUInt())
                stateFor(d, current.fileName, index, 0, 0)
            } ?: return@launch
            blocks.clear()
            looks.clear()
            blockAt.clear()
            pending.clear()
            screen = Screen.Sheet(state)
        }
    }

    fun selectCell(row: Int, col: Int) {
        val current = (screen as? Screen.Sheet)?.state ?: return
        screen = Screen.Sheet(current.copy(selectedRow = row, selectedCol = col))
        viewModelScope.launch {
            val edit = withContext(engine) {
                doc?.editText(current.sheet.toUInt(), row.toUInt(), col.toUInt())
            } ?: return@launch
            val now = (screen as? Screen.Sheet)?.state ?: return@launch
            if (now.selectedRow == row && now.selectedCol == col) {
                screen = Screen.Sheet(now.copy(editText = edit))
            }
        }
    }

    /** Enters [input] (a value or =formula) in the selected cell through GridCraft's `cell.set`. */
    fun commitCell(input: String) {
        val current = (screen as? Screen.Sheet)?.state ?: return
        edit { d -> d.setInput(current.selectedRow.toUInt(), current.selectedCol.toUInt(), input) }
    }

    /**
     * Runs GridCraft command [id] on the selected cell (its `range`, unless [params] names the
     * rows or columns), then shows the result.
     */
    fun format(id: String, params: JSONObject = JSONObject()) {
        val current = (screen as? Screen.Sheet)?.state ?: return
        if (!params.has("rows") && !params.has("cols")) params.put("range", cellName(current.selectedRow, current.selectedCol))
        edit { d -> d.execute(id, params.toString()) }
    }

    /** Inserts ([insert]) or deletes a whole row ([rows]) or column at the selection. */
    fun lines(rows: Boolean, insert: Boolean) {
        val current = (screen as? Screen.Sheet)?.state ?: return
        val at = if (rows) "${current.selectedRow + 1}" else columnName(current.selectedCol)
        val id = (if (insert) "home.insert" else "home.delete") + (if (rows) "Rows" else "Columns")
        edit { d -> d.execute(id, JSONObject().put(if (rows) "rows" else "cols", "$at:$at").toString()) }
    }

    /** Sorts the table by the selected column. */
    fun sort(ascending: Boolean) {
        val current = (screen as? Screen.Sheet)?.state ?: return
        edit { d -> d.sortColumn(current.selectedCol.toUInt(), ascending) }
    }

    /** Adds a sheet and shows it. */
    fun addSheet() = sheetEdit { d -> d.execute("home.insertSheet", "{}") }

    fun renameSheet(name: String) {
        val current = (screen as? Screen.Sheet)?.state ?: return
        sheetEdit { d -> d.execute("sheet.rename", JSONObject().put("sheet", current.sheet).put("name", name).toString()) }
    }

    /** Deletes the shown sheet (a workbook keeps at least one). */
    fun deleteSheet() {
        val current = (screen as? Screen.Sheet)?.state ?: return
        if (current.sheetNames.size < 2) return
        sheetEdit { d -> d.execute("home.deleteSheet", JSONObject().put("sheet", current.sheet).toString()) }
    }

    /** Like [edit], for changes that add, remove or rename sheets: shows the engine's active sheet afterwards. */
    private fun sheetEdit(op: (SheetDocument) -> Unit) {
        val current = (screen as? Screen.Sheet)?.state ?: return
        viewModelScope.launch {
            val result = withContext(engine) {
                runCatching {
                    val d = doc ?: error("no document")
                    op(d)
                    val sheet = d.activeSheet().toInt()
                    stateFor(d, current.fileName, sheet, 0, 0)
                }
            }
            result.fold(
                onSuccess = {
                    unsaved = true
                    blocks.clear()
                    looks.clear()
                    blockAt.clear()
                    pending.clear()
                    screen = Screen.Sheet(it)
                },
                onFailure = { message = text(R.string.cell_update_failed, it.message ?: "") },
            )
        }
    }

    /** Freezes the top row, the first column, or neither ([id]: view.freezeTopRow, view.freezeFirstColumn, view.unfreezePanes). */
    fun freeze(id: String) = edit { d -> d.execute(id, "{}") }

    fun undo() = edit { d -> d.execute("edit.undo", "{}") }

    fun redo() = edit { d -> d.execute("edit.redo", "{}") }

    /** Runs [op] on the engine thread, then refreshes the sheet; the old cells stay until the new ones arrive. */
    private fun edit(op: (SheetDocument) -> Unit) {
        val current = (screen as? Screen.Sheet)?.state ?: return
        viewModelScope.launch {
            val result = withContext(engine) {
                runCatching {
                    val d = doc ?: error("no document")
                    op(d)
                    stateFor(d, current.fileName, current.sheet, current.selectedRow, current.selectedCol)
                }
            }
            result.fold(
                onSuccess = {
                    unsaved = true
                    screen = Screen.Sheet(it)
                    // A formula, a new row or a format can change any cell: fetch the loaded ones again.
                    for ((key, at) in blockAt.toMap()) requestBlock(at.first, at.second, at.third, key)
                },
                onFailure = { message = text(R.string.cell_update_failed, it.message ?: "") },
            )
        }
    }

    /** Writes over the opened XLSX. */
    fun saveInPlace() {
        source?.let(::saveTo)
    }

    fun saveTo(uri: Uri) {
        val resolver = getApplication<Application>().contentResolver
        viewModelScope.launch {
            val result = withContext(engine) {
                runCatching {
                    val d = doc ?: error("no document")
                    val name = displayName(resolver, uri)
                    val bytes = d.save(name)
                    // "wt" truncates: without it some providers leave the old file's tail behind.
                    resolver.openOutputStream(uri, "wt")?.use { it.write(bytes) } ?: error("cannot write")
                }
            }
            message = result.fold(
                onSuccess = { text(R.string.saved) },
                onFailure = { text(R.string.save_failed, it.message ?: "") },
            )
            if (result.isSuccess) {
                unsaved = false
                if (closeAfterSave) close()
            }
            closeAfterSave = false
        }
    }

    /**
     * Writes the workbook as a PDF into the new file [uri]. [setup] goes into the sheets' Page
     * Setup through GridCraft's own commands (so a later "Save as" keeps it, as in Excel).
     */
    fun exportPdfTo(uri: Uri, setup: PdfSetup) {
        val resolver = getApplication<Application>().contentResolver
        val current = (screen as? Screen.Sheet)?.state ?: return
        viewModelScope.launch {
            val result = withContext(engine) {
                runCatching {
                    val d = doc ?: error("no document")
                    val sheets = if (setup.allSheets) current.sheetNames.indices else listOf(current.sheet)
                    for (i in sheets) {
                        d.setActiveSheet(i.toUInt())
                        d.execute("pageLayout.orientation", JSONObject().put("orientation", if (setup.landscape) "landscape" else "portrait").toString())
                        d.execute("pageLayout.size", JSONObject().put("paper", setup.paper).toString())
                        d.execute("pageLayout.margins", JSONObject().put("preset", setup.margins).toString())
                        d.execute("pageLayout.printGridlines", JSONObject().put("on", setup.gridlines).toString())
                    }
                    d.setActiveSheet(current.sheet.toUInt())
                    val range = setup.range.trim().takeIf { it.isNotEmpty() && !setup.allSheets }
                    val bytes = d.exportPdf(setup.allSheets, range, setup.fitToPage)
                    resolver.openOutputStream(uri, "wt")?.use { it.write(bytes) } ?: error("cannot write")
                }
            }
            message = result.fold(
                onSuccess = { text(R.string.saved) },
                onFailure = { text(R.string.save_failed, it.message ?: "") },
            )
        }
    }

    /** The text of a cell if its block is loaded; otherwise asks for the block and returns null. */
    fun cellText(sheet: Int, row: Int, col: Int): String? {
        val br = row / BLOCK_ROWS
        val bc = col / BLOCK_COLS
        val key = blockKey(sheet, br, bc)
        val block = blocks[key]
        if (block == null) {
            requestBlock(sheet, br, bc, key)
            return null
        }
        return block[(row - br * BLOCK_ROWS) * BLOCK_COLS + (col - bc * BLOCK_COLS)]
    }

    /** How a cell looks, if its block is loaded (see [cellText]). */
    fun cellLook(sheet: Int, row: Int, col: Int): CellLook? {
        val br = row / BLOCK_ROWS
        val bc = col / BLOCK_COLS
        return looks[blockKey(sheet, br, bc)]?.getOrNull((row - br * BLOCK_ROWS) * BLOCK_COLS + (col - bc * BLOCK_COLS))
    }

    private fun requestBlock(sheet: Int, br: Int, bc: Int, key: Long) {
        if (!pending.add(key)) return
        blockAt[key] = Triple(sheet, br, bc)
        viewModelScope.launch {
            val got = withContext(engine) {
                runCatching {
                    val d = doc ?: return@runCatching null
                    val (r, c) = (br * BLOCK_ROWS).toUInt() to (bc * BLOCK_COLS).toUInt()
                    d.cellTexts(sheet.toUInt(), r, c, BLOCK_ROWS.toUInt(), BLOCK_COLS.toUInt()) to
                        d.cellLooks(sheet.toUInt(), r, c, BLOCK_ROWS.toUInt(), BLOCK_COLS.toUInt())
                }.getOrNull()
            }
            pending.remove(key)
            if (got != null) {
                blocks[key] = got.first
                looks[key] = got.second
            }
        }
    }

    /** Runs on the engine thread. */
    private fun replaceDocument(next: SheetDocument): SheetUiState {
        doc?.close()
        doc = next
        // Numbers and formulas the Turkish way when the phone speaks Turkish ("1.250,5", =TOPLA(…;…)).
        next.setTurkish(Locale.getDefault().language == "tr")
        turkish = Locale.getDefault().language == "tr"
        functionNames = next.functionNames()
        blocks.clear()
        looks.clear()
        blockAt.clear()
        pending.clear()
        val sheet = next.activeSheet().toInt()
        return stateFor(next, "", sheet, 0, 0).copy(editText = next.editText(sheet.toUInt(), 0u, 0u))
    }

    /** Runs on the engine thread. */
    private fun stateFor(d: SheetDocument, fileName: String, sheet: Int, row: Int, col: Int): SheetUiState {
        val size = d.usedSize(sheet.toUInt())
        // Sizes for the used area and the room the grid scrolls past it; defaults beyond.
        val sizes = d.sizes(sheet.toUInt(), (size.cols.toInt() + 10).coerceAtMost(16_384).toUInt(), (size.rows.toInt() + 50).coerceAtMost(200_000).toUInt())
        return SheetUiState(
            fileName = fileName,
            sheetNames = d.sheetNames(),
            sheet = sheet,
            usedRows = size.rows.toInt(),
            usedCols = size.cols.toInt(),
            selectedRow = row,
            selectedCol = col,
            editText = d.editText(sheet.toUInt(), row.toUInt(), col.toUInt()),
            columns = GridAxis(sizes.colWidths.toFloatArray(), sizes.defaultCol),
            rows = GridAxis(sizes.rowHeights.toFloatArray(), sizes.defaultRow),
            frozenRows = sizes.frozenRows.toInt(),
            frozenCols = sizes.frozenCols.toInt(),
            merges = d.merges(sheet.toUInt()).map { it.toInt() }.chunked(4).map { it.toIntArray() },
        )
    }

    private fun text(id: Int, vararg args: Any): String = getApplication<Application>().getString(id, *args)

    override fun onCleared() {
        val old = doc
        doc = null
        old?.close()
    }

    companion object {
        const val BLOCK_ROWS = 64
        const val BLOCK_COLS = 16

        private fun blockKey(sheet: Int, br: Int, bc: Int): Long =
            (sheet.toLong() shl 48) or (br.toLong() shl 20) or bc.toLong()

        fun displayName(resolver: ContentResolver, uri: Uri): String =
            resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { c ->
                if (c.moveToFirst()) c.getString(0) else null
            } ?: uri.lastPathSegment?.substringAfterLast('/') ?: "document.xlsx"
    }
}

/** Column letters for a 0-based index: 0 → A, 25 → Z, 26 → AA. */
fun columnName(col: Int): String {
    val sb = StringBuilder()
    var n = col + 1
    while (n > 0) {
        val rem = (n - 1) % 26
        sb.append('A' + rem)
        n = (n - 1) / 26
    }
    return sb.reverse().toString()
}

fun cellName(row: Int, col: Int): String = columnName(col) + (row + 1)
