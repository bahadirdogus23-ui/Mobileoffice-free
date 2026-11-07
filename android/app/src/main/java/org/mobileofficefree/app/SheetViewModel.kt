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
import org.mobileofficefree.engine.FunctionDoc
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
    /** "none" (as the sheet says), "columns" (all columns one page wide) or "page" (each sheet on one page). */
    val fit: String = "columns",
    val gridlines: Boolean = false,
    /** Row 1 printed again at the top of every page. */
    val repeatHeader: Boolean = false,
    /** Row numbers and column letters printed. */
    val headings: Boolean = false,
    val centerH: Boolean = false,
    val pageNumbers: Boolean = true,
)

/** The first pages of a spreadsheet PDF, drawn small, and how many pages it has. */
class PdfPreview(val pages: Int, val images: List<android.graphics.Bitmap>)

data class SheetUiState(
    val fileName: String,
    val sheetNames: List<String>,
    val sheet: Int,
    /** Used area of the current sheet, counted from A1. */
    val usedRows: Int,
    val usedCols: Int,
    val selectedRow: Int = 0,
    val selectedCol: Int = 0,
    /** The far corner of the selected range (the selected cell itself when one cell is chosen). */
    val endRow: Int = selectedRow,
    val endCol: Int = selectedCol,
    /** A whole row or column was chosen by its header: merged cells do not widen it, as in Excel. */
    val wholeLine: Boolean = false,
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
    /** The selected cell's drop-down choices (Data Validation › List), if it has them. */
    val choices: List<String> = emptyList(),
    /** Where this sheet's pictures sit. */
    val pictures: List<org.mobileofficefree.engine.SheetChartBox> = emptyList(),
    /** Where this sheet's charts sit. */
    val charts: List<org.mobileofficefree.engine.SheetChartBox> = emptyList(),
    /** Notes and comments on this sheet's cells. */
    val notes: List<org.mobileofficefree.engine.SheetNote> = emptyList(),
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
    /** Every spreadsheet function, for hints and the formula help page. */
    var functionDocs by mutableStateOf<List<FunctionDoc>>(emptyList())
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

    // Auto-recovery: unsaved edits are copied every so often (see RecoveryStore).
    private val recovery = RecoveryStore(getApplication())

    /** A recovery copy left behind for the opened workbook: offered back once it opens. */
    var recoveryOffer by mutableStateOf<RecoveryStore.Copy?>(null)
        private set

    /** When the last recovery copy was made (0: none since the last save or open). */
    private var lastCopyAt = 0L

    private suspend fun copyForRecovery() {
        val uri = source ?: return
        val d = doc ?: return
        if (!unsaved) return
        lastCopyAt = System.currentTimeMillis()
        withContext(engine) { runCatching { recovery.write(uri, "recovery.xlsx", d.save("recovery.xlsx")) } }
    }

    private val copyNow: () -> Unit = { viewModelScope.launch { copyForRecovery() } }

    init {
        RecoveryHub.register(copyNow)
        viewModelScope.launch {
            // The first copy soon after the first edit, then one every RECOVERY_EVERY_MS.
            while (true) {
                kotlinx.coroutines.delay(RECOVERY_FIRST_MS)
                if (!unsaved) {
                    lastCopyAt = 0L
                    continue
                }
                if (lastCopyAt == 0L || System.currentTimeMillis() - lastCopyAt >= RECOVERY_EVERY_MS) copyForRecovery()
            }
        }
    }

    /** Takes the recovery copy back: the workbook as it was when last copied. */
    fun restoreRecovery() {
        val copy = recoveryOffer ?: return
        val current = (screen as? Screen.Sheet)?.state ?: return
        recoveryOffer = null
        viewModelScope.launch {
            val state = withContext(engine) { runCatching { replaceDocument(SheetDocument.open(copy.name, copy.file.readBytes(), null)) }.getOrNull() }
            if (state != null) {
                screen = Screen.Sheet(state.copy(fileName = current.fileName))
                // Recovered edits are not in the file yet.
                unsaved = true
            } else {
                message = text(R.string.recovery_failed)
            }
        }
    }

    /** Throws the recovery copy away (the person chose the file as it is). */
    fun discardRecovery() {
        recoveryOffer = null
        source?.let(recovery::clear)
    }

    /** Back or the arrow: closes, asking first when there are unsaved edits. */
    fun requestClose() {
        if (unsaved) askClose = true else close()
    }

    /** The answer to "save the changes?": [saveFirst] true means a "Save as" is starting, false discards, null stays. */
    fun answerClose(saveFirst: Boolean?) {
        askClose = false
        when (saveFirst) {
            true -> closeAfterSave = true
            false -> {
                // Thrown away on purpose: no copy to offer next time.
                source?.let(recovery::clear)
                close()
            }
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
                    recoveryOffer = recovery.find(uri)
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
        forgetCharts()
        forgetPictures()
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
            forgetCharts()
            forgetPictures()
            looks.clear()
            blockAt.clear()
            pending.clear()
            screen = Screen.Sheet(state)
        }
    }

    fun selectCell(row: Int, col: Int) {
        val current = (screen as? Screen.Sheet)?.state ?: return
        screen = Screen.Sheet(current.copy(selectedRow = row, selectedCol = col, endRow = row, endCol = col, choices = emptyList(), wholeLine = false))
        viewModelScope.launch {
            val (edit, choices) = withContext(engine) {
                val d = doc ?: return@withContext null
                d.editText(current.sheet.toUInt(), row.toUInt(), col.toUInt()) to runCatching { d.listChoices(row.toUInt(), col.toUInt()) }.getOrDefault(emptyList())
            } ?: return@launch
            val now = (screen as? Screen.Sheet)?.state ?: return@launch
            if (now.selectedRow == row && now.selectedCol == col) {
                screen = Screen.Sheet(now.copy(editText = edit, choices = choices))
            }
        }
    }

    /** Stretches the selection from the selected cell to (`row`, `col`). */
    fun extendSelection(row: Int, col: Int) {
        val current = (screen as? Screen.Sheet)?.state ?: return
        if (current.endRow == row && current.endCol == col) return
        screen = Screen.Sheet(current.copy(endRow = row, endCol = col))
    }

    /** Whether the selection is exactly one merged cell (so Merge would split it). */
    fun selectionIsMerged(): Boolean {
        val s = (screen as? Screen.Sheet)?.state ?: return false
        val b = selectionBounds(s)
        return s.merges.any { it[0] == b[0] && it[1] == b[1] && it[2] == b[2] && it[3] == b[3] }
    }

    /** Selects a whole row ([row]) or column ([col]) across the used area. */
    fun selectLine(row: Int?, col: Int?) {
        val current = (screen as? Screen.Sheet)?.state ?: return
        val lastRow = (current.usedRows - 1).coerceAtLeast(0)
        val lastCol = (current.usedCols - 1).coerceAtLeast(0)
        when {
            row != null -> {
                selectCell(row, 0)
                extendSelection(row, lastCol)
            }
            col != null -> {
                selectCell(0, col)
                extendSelection(lastRow, col)
            }
        }
        (screen as? Screen.Sheet)?.state?.let { screen = Screen.Sheet(it.copy(wholeLine = true)) }
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
        if (!params.has("rows") && !params.has("cols")) params.put("range", formatRange(current))
        edit { d -> d.execute(id, params.toString()) }
    }

    /** The text last copied from the sheet: a paste of the same text uses the engine's own copy (formulas and formats kept). */
    private var copiedText: String? = null

    /** Copies ([cut] false) or cuts the selection; [onText] gets it as tab-separated text for the phone's clipboard. */
    fun copySelection(cut: Boolean, onText: (String) -> Unit) {
        val current = (screen as? Screen.Sheet)?.state ?: return
        val range = formatRange(current)
        val run: (SheetDocument) -> Unit = { d ->
            val text = runCatching { JSONObject(d.execute(if (cut) "edit.cut" else "edit.copy", JSONObject().put("range", range).toString())).optString("text") }.getOrDefault("")
            viewModelScope.launch {
                copiedText = text
                if (text.isNotEmpty()) onText(text)
            }
        }
        if (cut) {
            edit(run)
        } else {
            val d = doc ?: return
            viewModelScope.launch { withContext(engine) { run(d) } }
        }
    }

    /**
     * Pastes at the selected cell: what was copied here (with its formulas and formats), or
     * [clipboard] (text from another app, tab-separated cells and lines) when that is newer.
     */
    fun paste(clipboard: String?) {
        val current = (screen as? Screen.Sheet)?.state ?: return
        val p = JSONObject().put("at", cellName(current.selectedRow, current.selectedCol))
        if (clipboard != null && clipboard.isNotEmpty() && clipboard != copiedText) p.put("text", clipboard)
        edit { d -> d.execute("edit.paste", p.toString()) }
    }

    /** Excel's AutoSum in the selected cell: [function] (SUM, AVERAGE, COUNT, MAX, MIN) of the numbers above or beside it. */
    fun autoSum(function: String) {
        val current = (screen as? Screen.Sheet)?.state ?: return
        val at = cellName(current.selectedRow, current.selectedCol)
        edit { d -> d.execute("formulas.autoSum", JSONObject().put("function", function).put("range", at).toString()) }
    }

    /**
     * A chart of [type] (column, bar, line, pie, area) made from the selected cells (or, with
     * one cell selected, the table around it), placed to the right of them.
     */
    fun insertChart(type: String) {
        val current = (screen as? Screen.Sheet)?.state ?: return
        val b = selectionBounds(current)
        val single = b[0] == b[2] && b[1] == b[3]
        // Beside the data: right of the selection, or of the whole used area for one cell.
        val col = if (single) maxOf(current.usedCols, b[3] + 1) else b[3] + 1
        // One cell picked: the sheet's used area is the data (a one-cell range would chart that cell alone).
        val range = if (single && current.usedRows > 0 && current.usedCols > 0) "A1:" + cellName(current.usedRows - 1, current.usedCols - 1) else formatRange(current)
        val p = JSONObject().put("type", type).put("range", range).put("at", cellName(if (single) 0 else b[0], col))
        edit { d -> d.execute("insert.chart", p.toString()) }
    }

    /** Adds a conditional format [rule] (GridCraft's rule JSON) to the selection. */
    fun conditionalFormat(rule: JSONObject) {
        val current = (screen as? Screen.Sheet)?.state ?: return
        val range = formatRange(current)
        edit { d -> d.execute("home.conditionalFormat", JSONObject().put("range", range).put("rule", rule).toString()) }
    }

    /** Removes the conditional formats from the selection. */
    fun clearConditionalFormats() {
        val current = (screen as? Screen.Sheet)?.state ?: return
        edit { d -> d.execute("home.clearRules", JSONObject().put("range", formatRange(current)).toString()) }
    }

    /** Charts drawn so far, by (sheet, chart index); emptied when the data may have changed. */
    private val chartPictures = mutableStateMapOf<Pair<Int, Int>, android.graphics.Bitmap>()
    private val chartsAsked = mutableSetOf<Pair<Int, Int>>()

    /** Chart [index] of [sheet] as a picture, or null while it is being drawn (then asked for). */
    fun chartPicture(sheet: Int, index: Int): android.graphics.Bitmap? {
        val key = sheet to index
        chartPictures[key]?.let { return it }
        if (!chartsAsked.add(key)) return null
        val d = doc ?: return null
        viewModelScope.launch {
            val bmp = withContext(engine) { runCatching { drawChart(d.renderChart(sheet.toUInt(), index.toUInt(), CHART_SCALE)) }.getOrNull() }
            if (bmp != null) chartPictures[key] = bmp
        }
        return null
    }

    /** Pictures decoded so far, by (sheet, picture index). */
    private val sheetPictures = mutableStateMapOf<Pair<Int, Int>, android.graphics.Bitmap>()
    private val picturesAsked = mutableSetOf<Pair<Int, Int>>()

    /** Picture [index] of [sheet] decoded (at most 2048 pixels on its long side), or null while it is. */
    fun sheetPicture(sheet: Int, index: Int): android.graphics.Bitmap? {
        val key = sheet to index
        sheetPictures[key]?.let { return it }
        if (!picturesAsked.add(key)) return null
        val d = doc ?: return null
        viewModelScope.launch {
            val bmp = withContext(engine) {
                runCatching {
                    val bytes = d.pictureBytes(sheet.toUInt(), index.toUInt())
                    val bounds = android.graphics.BitmapFactory.Options().apply { inJustDecodeBounds = true }
                    android.graphics.BitmapFactory.decodeByteArray(bytes, 0, bytes.size, bounds)
                    var sample = 1
                    while (maxOf(bounds.outWidth, bounds.outHeight) / (sample * 2) >= 2048) sample *= 2
                    android.graphics.BitmapFactory.decodeByteArray(bytes, 0, bytes.size, android.graphics.BitmapFactory.Options().apply { inSampleSize = sample })
                }.getOrNull()
            }
            if (bmp != null) sheetPictures[key] = bmp
        }
        return null
    }

    /** Pictures to be decoded again (another document, or sheets added or removed). */
    private fun forgetPictures() {
        sheetPictures.clear()
        picturesAsked.clear()
    }

    /** Charts to be drawn again (after an edit, the data under them may differ). */
    private fun forgetCharts() {
        chartPictures.clear()
        chartsAsked.clear()
    }

    /** The sheet as shown now, when one is. */
    val sheetState: SheetUiState? get() = (screen as? Screen.Sheet)?.state

    /** Gives the selected cell the note [text] (replacing one it has); an empty text removes it. */
    fun setNote(text: String) {
        val current = (screen as? Screen.Sheet)?.state ?: return
        val at = cellName(current.selectedRow, current.selectedCol)
        val had = current.notes.any { it.row.toInt() == current.selectedRow && it.col.toInt() == current.selectedCol }
        // Signed with the name from Settings › My details (none rather than a placeholder).
        val author = Profile.get(getApplication()).name
        edit { d ->
            if (had) d.execute("review.deleteComment", JSONObject().put("cell", at).toString())
            if (text.isNotBlank()) d.execute("review.newNote", JSONObject().put("cell", at).put("text", text.trim()).put("author", author).toString())
        }
    }

    /** Empties the selected cells (their formats stay). */
    fun clearSelection() {
        val current = (screen as? Screen.Sheet)?.state ?: return
        edit { d -> d.execute("edit.clearContents", JSONObject().put("range", formatRange(current)).toString()) }
    }

    /** Copies the selection's first row down through the rest of it, as Excel's Fill Down. */
    fun fillDown() {
        val current = (screen as? Screen.Sheet)?.state ?: return
        edit { d -> d.execute("edit.fillDown", JSONObject().put("range", formatRange(current)).toString()) }
    }

    /**
     * Column [col] (when ≥ 0) or row [row] made [size] wide or tall (dragged by its header's
     * edge), in the grid's unit: Excel's pixels, as GridCraft gives and takes sizes.
     */
    fun resizeLine(col: Int, row: Int, size: Float) {
        val px = size
        val p = if (col >= 0) {
            JSONObject().put("cols", "${columnName(col)}:${columnName(col)}").put("width", px.toDouble())
        } else {
            JSONObject().put("rows", "${row + 1}:${row + 1}").put("height", px.toDouble())
        }
        edit { d -> d.execute(if (col >= 0) "home.columnWidth" else "home.rowHeight", p.toString()) }
    }

    /** The selected rows ("2:5") or columns ("B:D"), for whole-line commands. */
    private fun selectedLines(s: SheetUiState, rows: Boolean): String {
        val (r0, c0, r1, c1) = selectionBounds(s)
        return if (rows) "${r0 + 1}:${r1 + 1}" else "${columnName(c0)}:${columnName(c1)}"
    }

    /** Hides the selected rows ([rows]) or columns. */
    fun hideLines(rows: Boolean) {
        val current = (screen as? Screen.Sheet)?.state ?: return
        val p = JSONObject().put(if (rows) "rows" else "cols", selectedLines(current, rows))
        edit { d -> d.execute(if (rows) "home.hideRows" else "home.hideColumns", p.toString()) }
    }

    /** Shows every hidden row and column of the sheet again. */
    fun unhideAll() = edit { d ->
        d.execute("home.unhideRows", JSONObject().put("rows", "1:1048576").toString())
        d.execute("home.unhideColumns", JSONObject().put("cols", "A:XFD").toString())
    }

    /** The selected columns as wide as their contents. */
    fun autofitColumns() {
        val current = (screen as? Screen.Sheet)?.state ?: return
        edit { d -> d.execute("home.autofitColumnWidth", JSONObject().put("cols", selectedLines(current, false)).toString()) }
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

    /** Find: the cells found, which one is shown, and a counter the grid scrolls on. */
    var findHits by mutableStateOf<List<org.mobileofficefree.engine.SheetCellHit>>(emptyList())
        private set
    var findIndex by mutableStateOf(-1)
        private set
    var reveal by mutableStateOf(0)
        private set

    fun find(query: String) {
        val d = doc ?: return
        viewModelScope.launch {
            val hits = withContext(engine) { runCatching { d.findCells(query) }.getOrDefault(emptyList()) }
            findHits = hits
            findIndex = -1
            if (hits.isNotEmpty()) showHit(0)
        }
    }

    /** Shows the next ([step] 1) or previous (-1) cell found. */
    fun nextHit(step: Int) {
        if (findHits.isEmpty()) return
        showHit(((findIndex + step) % findHits.size + findHits.size) % findHits.size)
    }

    private fun showHit(i: Int) {
        val hit = findHits[i]
        findIndex = i
        goToCell(hit.sheet.toInt(), hit.row.toInt(), hit.col.toInt())
    }

    /** Selects cell (`row`, `col`) of [sheet] and scrolls it into view. */
    fun goToCell(sheet: Int, row: Int, col: Int) {
        val current = (screen as? Screen.Sheet)?.state ?: return
        if (current.sheet == sheet) {
            selectCell(row, col)
            reveal++
            return
        }
        viewModelScope.launch {
            val state = withContext(engine) {
                val d = doc ?: return@withContext null
                d.setActiveSheet(sheet.toUInt())
                stateFor(d, current.fileName, sheet, row, col)
            } ?: return@launch
            blocks.clear()
            forgetCharts()
            forgetPictures()
            looks.clear()
            blockAt.clear()
            pending.clear()
            screen = Screen.Sheet(state)
            reveal++
        }
    }

    fun endFind() {
        findHits = emptyList()
        findIndex = -1
    }

    /** Replaces [find] with [with] in every typed cell and tells how many changed. */
    fun replaceAll(find: String, with: String) {
        edit { d ->
            val n = d.replaceAll(find, with).toInt()
            val text = getApplication<Application>().resources.getQuantityString(R.plurals.replaced, n, n)
            viewModelScope.launch {
                message = text
                findHits = emptyList()
                findIndex = -1
            }
        }
    }

    /** The selected column's values for the filter sheet, or null while closed. */
    var filterValues by mutableStateOf<List<org.mobileofficefree.engine.SheetFilterValue>?>(null)
        private set
    var filterColumn by mutableStateOf(0)
        private set
    var hasFilter by mutableStateOf(false)
        private set

    /** Opens the filter sheet for the selected column. */
    fun openFilter() {
        val current = (screen as? Screen.Sheet)?.state ?: return
        val d = doc ?: return
        viewModelScope.launch {
            val (values, on) = withContext(engine) {
                runCatching { d.filterValues(current.selectedCol.toUInt()) }.getOrNull() to runCatching { d.hasFilter() }.getOrDefault(false)
            }
            hasFilter = on
            filterColumn = current.selectedCol
            filterValues = values.orEmpty()
        }
    }

    fun closeFilter() {
        filterValues = null
    }

    /** Shows only the rows whose filtered column holds one of [values] ("" for blanks). */
    fun applyFilter(values: List<String>) {
        val col = filterColumn
        filterValues = null
        edit { d ->
            d.filterColumn(col.toUInt(), values)
            val on = d.hasFilter()
            viewModelScope.launch { hasFilter = on }
        }
    }

    fun removeFilter() {
        filterValues = null
        edit { d ->
            d.removeFilter()
            viewModelScope.launch { hasFilter = false }
        }
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
                    forgetCharts()
                    forgetPictures()
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

    /** Freezes the rows above and the columns left of the selected cell, as Excel's Freeze Panes. */
    fun freezeAtSelection() {
        val current = (screen as? Screen.Sheet)?.state ?: return
        val at = cellName(current.selectedRow, current.selectedCol)
        edit { d -> d.execute("view.freezePanes", JSONObject().put("cell", at).toString()) }
    }

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
                        .copy(endRow = current.endRow, endCol = current.endCol)
                }
            }
            result.fold(
                onSuccess = {
                    unsaved = true
                    // The selection may have moved while the engine worked (Enter in a cell).
                    val now = (screen as? Screen.Sheet)?.state
                    screen = Screen.Sheet(
                        if (now != null) it.copy(selectedRow = now.selectedRow, selectedCol = now.selectedCol, endRow = now.endRow, endCol = now.endCol, editText = if (now.selectedRow == current.selectedRow && now.selectedCol == current.selectedCol) it.editText else now.editText) else it,
                    )
                    // A formula, a new row or a format can change any cell: fetch the loaded ones again.
                    for ((key, at) in blockAt.toMap()) requestBlock(at.first, at.second, at.third, key)
                    forgetCharts()
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
                source?.let(recovery::clear)
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
                    val bytes = pdfWith(d, setup, current)
                    resolver.openOutputStream(uri, "wt")?.use { it.write(bytes) } ?: error("cannot write")
                }
            }
            message = result.fold(
                onSuccess = { text(R.string.saved) },
                onFailure = { text(R.string.save_failed, it.message ?: "") },
            )
        }
    }

    /** Runs on the engine thread: [setup] put into the sheets' Page Setup, then the PDF. */
    private fun pdfWith(d: SheetDocument, setup: PdfSetup, current: SheetUiState): ByteArray {
        val sheets = if (setup.allSheets) current.sheetNames.indices else listOf(current.sheet)
        val footer = "&C" + getApplication<Application>().getString(R.string.page_word) + " &P / &N"
        for (i in sheets) {
            d.setActiveSheet(i.toUInt())
            val page = JSONObject()
                .put("orientation", if (setup.landscape) "landscape" else "portrait")
                .put("paper", setup.paper)
                .put("gridlines", setup.gridlines)
                .put("headings", setup.headings)
                .put("centerH", setup.centerH)
                .put("titleRows", if (setup.repeatHeader) "1:1" else "")
            when (setup.fit) {
                "columns" -> page.put("fitWidth", 1).put("fitHeight", 0)
                "page" -> page.put("fitToPage", true)
                else -> page.put("fitToPage", false)
            }
            if (setup.pageNumbers) page.put("footer", footer)
            d.execute("pageLayout.pageSetup", page.toString())
            d.execute("pageLayout.margins", JSONObject().put("preset", setup.margins).toString())
        }
        d.setActiveSheet(current.sheet.toUInt())
        val range = setup.range.trim().takeIf { it.isNotEmpty() && !setup.allSheets }
        return d.exportPdf(setup.allSheets, range, setup.fit == "page")
    }

    /** The PDF [setup] makes, its first pages drawn [widthPx] wide, for the Page Setup sheet. */
    suspend fun previewPdf(setup: PdfSetup, widthPx: Int): PdfPreview? {
        val current = (screen as? Screen.Sheet)?.state ?: return null
        val cache = java.io.File(getApplication<Application>().cacheDir, "sheet-preview.pdf")
        return withContext(engine) {
            runCatching {
                val d = doc ?: error("no document")
                cache.writeBytes(pdfWith(d, setup, current))
                android.os.ParcelFileDescriptor.open(cache, android.os.ParcelFileDescriptor.MODE_READ_ONLY).use { fd ->
                    android.graphics.pdf.PdfRenderer(fd).use { r ->
                        val images = (0 until minOf(r.pageCount, 4)).map { i ->
                            r.openPage(i).use { pg ->
                                val h = (widthPx.toFloat() * pg.height / pg.width).toInt().coerceAtLeast(1)
                                android.graphics.Bitmap.createBitmap(widthPx, h, android.graphics.Bitmap.Config.ARGB_8888).also { b ->
                                    b.eraseColor(android.graphics.Color.WHITE)
                                    pg.render(b, null, null, android.graphics.pdf.PdfRenderer.Page.RENDER_MODE_FOR_DISPLAY)
                                }
                            }
                        }
                        PdfPreview(r.pageCount, images)
                    }
                }
            }.getOrNull()
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
        functionDocs = next.functionDocs()
        hasFilter = next.hasFilter()
        blocks.clear()
        forgetCharts()
        forgetPictures()
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
            notes = runCatching { d.notes(sheet.toUInt()) }.getOrDefault(emptyList()),
            charts = runCatching { d.charts(sheet.toUInt()) }.getOrDefault(emptyList()),
            pictures = runCatching { d.pictures(sheet.toUInt()) }.getOrDefault(emptyList()),
            choices = runCatching { d.listChoices(row.toUInt(), col.toUInt()) }.getOrDefault(emptyList()),
        )
    }

    private fun text(id: Int, vararg args: Any): String = getApplication<Application>().getString(id, *args)

    override fun onCleared() {
        RecoveryHub.unregister(copyNow)
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

/** The selection as A1 text: one cell ("B3") or a range ("B3:D9"). */
/**
 * The selection as [first row, first column, last row, last column], grown to take in every
 * merged range it touches, as Excel's is: what formatting applies to.
 */
fun selectionBounds(s: SheetUiState): IntArray {
    var r0 = minOf(s.selectedRow, s.endRow)
    var r1 = maxOf(s.selectedRow, s.endRow)
    var c0 = minOf(s.selectedCol, s.endCol)
    var c1 = maxOf(s.selectedCol, s.endCol)
    var grown = !s.wholeLine
    while (grown) {
        grown = false
        for (m in s.merges) {
            val touches = m[0] <= r1 && m[2] >= r0 && m[1] <= c1 && m[3] >= c0
            if (touches && (m[0] < r0 || m[2] > r1 || m[1] < c0 || m[3] > c1)) {
                r0 = minOf(r0, m[0]); r1 = maxOf(r1, m[2]); c0 = minOf(c0, m[1]); c1 = maxOf(c1, m[3])
                grown = true
            }
        }
    }
    return intArrayOf(r0, c0, r1, c1)
}

/** The range formatting applies to ("B2", "A1:D4"): the selection with the merged ranges it touches. */
fun formatRange(s: SheetUiState): String {
    val (r0, c0, r1, c1) = selectionBounds(s)
    return if (r0 == r1 && c0 == c1) cellName(r0, c0) else cellName(r0, c0) + ":" + cellName(r1, c1)
}

fun rangeName(s: SheetUiState): String {
    val r0 = minOf(s.selectedRow, s.endRow)
    val r1 = maxOf(s.selectedRow, s.endRow)
    val c0 = minOf(s.selectedCol, s.endCol)
    val c1 = maxOf(s.selectedCol, s.endCol)
    return if (r0 == r1 && c0 == c1) cellName(r0, c0) else cellName(r0, c0) + ":" + cellName(r1, c1)
}

/** How many pixels per grid unit charts are drawn at (sharp when zoomed in a little). */
private const val CHART_SCALE = 2f

/** A drawn chart as a bitmap: its shapes, then its words written in the phone's own type. */
private fun drawChart(p: org.mobileofficefree.engine.SheetChartPicture): android.graphics.Bitmap {
    val w = p.width.toInt()
    val h = p.height.toInt()
    // Opaque straight RGBA is ARGB_8888's memory layout.
    val bmp = android.graphics.Bitmap.createBitmap(w, h, android.graphics.Bitmap.Config.ARGB_8888)
    bmp.copyPixelsFromBuffer(java.nio.ByteBuffer.wrap(p.pixels))
    val canvas = android.graphics.Canvas(bmp)
    val paint = android.graphics.Paint(android.graphics.Paint.ANTI_ALIAS_FLAG)
    for (t in p.texts) {
        paint.color = t.color.toInt()
        paint.textSize = t.size
        paint.typeface = if (t.bold) android.graphics.Typeface.DEFAULT_BOLD else android.graphics.Typeface.DEFAULT
        paint.textAlign = when (t.align.toInt()) {
            1 -> android.graphics.Paint.Align.CENTER
            2 -> android.graphics.Paint.Align.RIGHT
            else -> android.graphics.Paint.Align.LEFT
        }
        val fm = paint.fontMetrics
        // The baseline that puts the text's top, middle or bottom at y.
        val dy = when (t.valign.toInt()) {
            0 -> -fm.ascent
            1 -> -(fm.ascent + fm.descent) / 2
            else -> -fm.descent
        }
        canvas.save()
        canvas.rotate(Math.toDegrees(t.rotation.toDouble()).toFloat(), t.x, t.y)
        canvas.drawText(t.text, t.x, t.y + dy, paint)
        canvas.restore()
    }
    return bmp
}
