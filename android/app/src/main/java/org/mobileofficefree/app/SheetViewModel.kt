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
import org.mobileofficefree.engine.SheetDocument
import org.mobileofficefree.engine.nativeVersion

/** Everything the spreadsheet screen shows, apart from the cell texts (see [SheetViewModel.cellText]). */
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
    private val pending = HashSet<Long>()

    val engineVersion: String by lazy { runCatching { nativeVersion() }.getOrDefault("?") }

    fun consumeMessage() {
        message = null
    }

    fun dismissImportNotes() {
        importNotes = emptyList()
    }

    fun open(uri: Uri) {
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
        screen = Screen.Loading
        val started = SystemClock.elapsedRealtime()
        viewModelScope.launch {
            val name = text(R.string.new_workbook_name)
            screen = Screen.Sheet(withContext(engine) { replaceDocument(SheetDocument.blank()) }.copy(fileName = name))
        }
    }

    fun close() {
        val old = doc
        doc = null
        blocks.clear()
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
        val params = JSONObject()
            .put("cell", cellName(current.selectedRow, current.selectedCol))
            .put("input", input)
            .toString()
        viewModelScope.launch {
            val result = withContext(engine) {
                runCatching {
                    val d = doc ?: error("no document")
                    d.execute("cell.set", params)
                    stateFor(d, current.fileName, current.sheet, current.selectedRow, current.selectedCol)
                }
            }
            result.fold(
                onSuccess = {
                    // A formula can change any cell: refetch everything visible.
                    blocks.clear()
                    pending.clear()
                    screen = Screen.Sheet(it)
                },
                onFailure = { message = text(R.string.cell_update_failed, it.message ?: "") },
            )
        }
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
        }
    }

    /** Writes the workbook as a PDF (every sheet, each with its Page Setup) into the new file [uri]. */
    fun exportPdfTo(uri: Uri) {
        val resolver = getApplication<Application>().contentResolver
        viewModelScope.launch {
            val result = withContext(engine) {
                runCatching {
                    val bytes = (doc ?: error("no document")).exportPdf(true)
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

    private fun requestBlock(sheet: Int, br: Int, bc: Int, key: Long) {
        if (!pending.add(key)) return
        viewModelScope.launch {
            val texts = withContext(engine) {
                runCatching {
                    doc?.cellTexts(
                        sheet.toUInt(),
                        (br * BLOCK_ROWS).toUInt(),
                        (bc * BLOCK_COLS).toUInt(),
                        BLOCK_ROWS.toUInt(),
                        BLOCK_COLS.toUInt(),
                    )
                }.getOrNull()
            }
            pending.remove(key)
            if (texts != null) blocks[key] = texts
        }
    }

    /** Runs on the engine thread. */
    private fun replaceDocument(next: SheetDocument): SheetUiState {
        doc?.close()
        doc = next
        blocks.clear()
        pending.clear()
        val sheet = next.activeSheet().toInt()
        return stateFor(next, "", sheet, 0, 0).copy(editText = next.editText(sheet.toUInt(), 0u, 0u))
    }

    /** Runs on the engine thread. */
    private fun stateFor(d: SheetDocument, fileName: String, sheet: Int, row: Int, col: Int): SheetUiState {
        val size = d.usedSize(sheet.toUInt())
        return SheetUiState(
            fileName = fileName,
            sheetNames = d.sheetNames(),
            sheet = sheet,
            usedRows = size.rows.toInt(),
            usedCols = size.cols.toInt(),
            selectedRow = row,
            selectedCol = col,
            editText = d.editText(sheet.toUInt(), row.toUInt(), col.toUInt()),
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
