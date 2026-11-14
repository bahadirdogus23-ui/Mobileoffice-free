package org.mobileofficefree.app

import androidx.compose.material3.Surface
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.sp
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.material3.Checkbox
import androidx.compose.foundation.Image
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.PrimaryScrollableTabRow
import androidx.compose.material3.Switch
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.dp
import java.util.Locale
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.clickable
import androidx.compose.foundation.selection.toggleable
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextOverflow
import org.mobileofficefree.engine.FunctionDoc
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.draw.shadow
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.graphics.Color
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.ui.draw.clip
import androidx.compose.material3.OutlinedButton
import org.json.JSONObject
import org.mobileofficefree.engine.CellLook

/** Spreadsheet types the picker offers (the engine also sniffs content, so a wrong type still opens). */
internal val SPREADSHEET_TYPES = arrayOf(
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "application/vnd.ms-excel.sheet.macroenabled.12",
    "application/vnd.ms-excel",
    "application/vnd.ms-excel.sheet.binary.macroenabled.12",
    "application/vnd.oasis.opendocument.spreadsheet",
    "text/csv",
    "text/comma-separated-values",
    "text/tab-separated-values",
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun SheetScreen(vm: SheetViewModel, state: SheetUiState) {
    val saveAs = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument(SPREADSHEET_TYPES[0]),
    ) { uri -> if (uri != null) vm.saveTo(uri) else vm.saveCancelled() }
    var pdfSetup by remember { mutableStateOf(PdfSetup()) }
    var finding by remember { mutableStateOf(false) }
    var showPdf by remember { mutableStateOf(false) }
    // A keyboard: Ctrl+S saves, Ctrl+F finds, Ctrl+Z/Y undo and redo, Ctrl+P makes the PDF.
    val activity = androidx.activity.compose.LocalActivity.current as? MainActivity
    androidx.compose.runtime.DisposableEffect(activity, state.fileName) {
        activity?.shortcutHandler = { key ->
            when (key) {
                Shortcut.SAVE -> {
                    if (vm.canSaveInPlace) vm.saveInPlace() else saveAs.launch(saveName(state.fileName))
                    true
                }
                Shortcut.FIND -> { finding = true; true }
                Shortcut.UNDO -> { vm.undo(); true }
                Shortcut.REDO -> { vm.redo(); true }
                Shortcut.PRINT -> { showPdf = true; true }
                else -> false
            }
        }
        onDispose { activity?.shortcutHandler = null }
    }
    var sheetMenu by remember { mutableStateOf(false) }
    var renaming by remember { mutableStateOf<String?>(null) }
    var confirmDelete by remember { mutableStateOf(false) }
    val savePdf = rememberLauncherForActivityResult(CreateFile()) { uri -> if (uri != null) vm.exportPdfTo(uri, pdfSetup) }
    // The formula bar's text, kept here so that a tap on a cell can go into a formula being typed.
    var formula by remember(state.sheet, state.selectedRow, state.selectedCol, state.editText) {
        mutableStateOf(TextFieldValue(state.editText, TextRange(state.editText.length)))
    }
    var formulaFocused by remember { mutableStateOf(false) }
    // Typing straight into the selected cell (a second tap on it); ends with Enter or another cell.
    var cellEditing by remember(state.sheet) { mutableStateOf(false) }
    var lastReference by remember(state.sheet, state.selectedRow, state.selectedCol) { mutableStateOf<TextRange?>(null) }
    var showFormulas by remember { mutableStateOf(false) }
    if (showFormulas) {
        FormulaHelpSheet(vm.functionDocs, onDismiss = { showFormulas = false }) { name ->
            showFormulas = false
            val base = if (formula.text.isEmpty()) TextFieldValue("=", TextRange(1)) else formula
            formula = typeAt(base, "$name(")
        }
    }

    Column(Modifier.fillMaxSize().navigationBarsPadding()) {
        TopAppBar(
            title = { Text(state.fileName, maxLines = 1, overflow = TextOverflow.Ellipsis) },
            navigationIcon = {
                IconButton(onClick = vm::requestClose) {
                    Icon(painterResource(R.drawable.ic_arrow_back), contentDescription = stringResource(R.string.back))
                }
            },
            actions = {
                if (vm.canSaveInPlace && vm.hasUnsaved) {
                    TextButton(onClick = vm::saveInPlace) { Text(stringResource(R.string.save)) }
                }
                TextButton(onClick = { finding = !finding }, modifier = Modifier.described(stringResource(R.string.a11y_find))) { Text("🔍") }
                // PDF and "Save as" sit behind ⋮: with large text (and Turkish's longer words) they
                // used to take the whole bar and squeeze the file name out of sight.
                var more by remember { mutableStateOf(false) }
                Box {
                    IconButton(onClick = { more = true }, modifier = Modifier.described(stringResource(R.string.a11y_more))) { Text("⋮", fontSize = 22.sp) }
                    DropdownMenu(expanded = more, onDismissRequest = { more = false }) {
                        DropdownMenuItem(text = { Text(stringResource(R.string.save_as)) }, onClick = {
                            more = false
                            saveAs.launch(saveName(state.fileName))
                        })
                        DropdownMenuItem(text = { Text("PDF") }, onClick = {
                            more = false
                            showPdf = true
                        })
                    }
                }
            },
        )
        FormulaBar(
            state = state,
            value = formula,
            onValue = { formula = it },
            onFocus = { formulaFocused = it },
            focused = formulaFocused,
            docs = vm.functionDocs,
            turkish = vm.turkish,
            onAllFormulas = { showFormulas = true },
            onCommit = vm::commitCell,
        )
        // The selected cell's drop-down list, as Excel's ▾ button opens it.
        if (state.choices.isNotEmpty()) {
            Row(
                Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 8.dp, vertical = 2.dp),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text("▾", color = MaterialTheme.colorScheme.primary, modifier = Modifier.decorative())
                for (choice in state.choices.take(100)) {
                    FilterChip(selected = choice == state.editText, onClick = { vm.commitCell(choice) }, label = { Text(choice, maxLines = 1) })
                }
            }
        }
        // The selected cell's note, as Excel shows it on hover.
        state.notes.firstOrNull { it.row.toInt() == state.selectedRow && it.col.toInt() == state.selectedCol }?.let { note ->
            Surface(color = Color(0xFFFFF8C4), modifier = Modifier.fillMaxWidth()) {
                Text(
                    (if (note.author.isNotBlank()) note.author + ": " else "") + note.text,
                    style = MaterialTheme.typography.bodySmall,
                    color = Color(0xFF3E3A1A),
                    maxLines = 4,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
                )
            }
        }
        if (finding) {
            FindBar(vm, onClose = {
                finding = false
                vm.endFind()
            })
        } else {
            SheetToolBar(vm, vm.cellLook(state.sheet, state.selectedRow, state.selectedCol))
        }
        HorizontalDivider()
        SheetGrid(
            sheet = state.sheet,
            usedRows = state.usedRows,
            usedCols = state.usedCols,
            selectedRow = state.selectedRow,
            selectedCol = state.selectedCol,
            endRow = state.endRow,
            endCol = state.endCol,
            cellText = vm::cellText,
            cellLook = vm::cellLook,
            columnAxis = state.columns,
            rowAxis = state.rows,
            frozenRows = state.frozenRows,
            frozenCols = state.frozenCols,
            merges = state.merges,
            reveal = vm.reveal,
            charts = state.pictures.map { it to vm.sheetPicture(state.sheet, it.index.toInt()) } +
                state.charts.map { it to vm.chartPicture(state.sheet, it.index.toInt()) },
            noteCells = remember(state.notes) { state.notes.map { it.row.toLong() * 16384 + it.col.toLong() }.toSet() },
            onResize = vm::resizeLine,
            wholeLine = state.wholeLine,
            onChartTap = { vm.pickChart(it) },
            pickedChart = vm.pickedChart,
            formulaRefs = if (formulaFocused || cellEditing) remember(formula.text) { formulaReferences(formula.text) } else emptyList(),
            onSelect = { row, col ->
                if (vm.placeChart(row, col)) {
                    // The tap put the picked chart there.
                } else if ((formulaFocused || cellEditing) && formula.text.startsWith("=")) {
                    // A tap inside a quoted formula must neither insert an address nor leave
                    // the editor and attempt to save an unfinished string or sheet name.
                    if (pointing(formula)) {
                        val (v, r) = insertReference(formula, cellName(row, col), lastReference)
                        formula = v
                        lastReference = r
                    }
                } else {
                    if (cellEditing) {
                        // Leaving a cell being typed in keeps what was typed, as Excel does.
                        if (formula.text != state.editText) vm.commitCell(formula.text)
                        cellEditing = false
                    }
                    vm.selectCell(row, col)
                }
            },
            onExtend = vm::extendSelection,
            onLine = vm::selectLine,
            onEditCell = { cellEditing = true },
            cellEditor = if (cellEditing) {
                { m ->
                    CellEditor(
                        value = formula,
                        onValue = { formula = it },
                        onFocus = { formulaFocused = it },
                        onDone = {
                            vm.commitCell(formula.text)
                            cellEditing = false
                            // Enter moves down, as in Excel.
                            // Below a merged cell is the row after the whole merged range.
                            val merged = state.merges.firstOrNull { it[0] == state.selectedRow && it[1] == state.selectedCol }
                            val below = (merged?.get(2) ?: state.selectedRow) + 1
                            val next = state.merges.firstOrNull { below in it[0]..it[2] && state.selectedCol in it[1]..it[3] }
                            vm.selectCell(next?.get(0) ?: below, next?.get(1) ?: state.selectedCol)
                        },
                        modifier = m,
                    )
                }
            } else {
                null
            },
            modifier = Modifier.weight(1f),
        )
        Row(verticalAlignment = Alignment.CenterVertically) {
            PrimaryScrollableTabRow(selectedTabIndex = state.sheet, modifier = Modifier.weight(1f)) {
                state.sheetNames.forEachIndexed { i, name ->
                    Tab(selected = i == state.sheet, onClick = { vm.selectSheet(i) }, text = { Text(name) })
                }
            }
            Box {
                TextButton(onClick = { sheetMenu = true }) { Text(stringResource(R.string.sheet_menu)) }
                DropdownMenu(expanded = sheetMenu, onDismissRequest = { sheetMenu = false }) {
                    DropdownMenuItem(text = { Text(stringResource(R.string.sheet_add)) }, onClick = {
                        sheetMenu = false
                        vm.addSheet()
                    })
                    DropdownMenuItem(
                        text = { Text(stringResource(R.string.freeze_at_cell, cellName(state.selectedRow, state.selectedCol))) },
                        enabled = state.selectedRow > 0 || state.selectedCol > 0,
                        onClick = {
                            sheetMenu = false
                            vm.freezeAtSelection()
                        },
                    )
                    for ((label, id) in listOf(
                        R.string.freeze_top_row to "view.freezeTopRow",
                        R.string.freeze_first_col to "view.freezeFirstColumn",
                        R.string.unfreeze to "view.unfreezePanes",
                    )) {
                        DropdownMenuItem(text = { Text(stringResource(label)) }, onClick = {
                            sheetMenu = false
                            vm.freeze(id)
                        })
                    }
                    DropdownMenuItem(text = { Text(stringResource(R.string.sheet_rename)) }, onClick = {
                        sheetMenu = false
                        renaming = state.sheetNames.getOrElse(state.sheet) { "" }
                    })
                    DropdownMenuItem(
                        text = { Text(stringResource(R.string.sheet_delete)) },
                        enabled = state.sheetNames.size > 1,
                        onClick = {
                            sheetMenu = false
                            confirmDelete = true
                        },
                    )
                }
            }
        }
    }

    if (vm.pickedChart != null && !vm.movingChart) ChartMenu(vm)

    vm.recoveryOffer?.let { copy ->
        val time = remember(copy) { java.text.DateFormat.getTimeInstance(java.text.DateFormat.SHORT).format(java.util.Date(copy.savedAt)) }
        AlertDialog(
            onDismissRequest = {},
            title = { Text(stringResource(R.string.recovery_title)) },
            text = { Text(stringResource(R.string.recovery_text, time)) },
            confirmButton = { TextButton(onClick = vm::restoreRecovery) { Text(stringResource(R.string.recovery_restore)) } },
            dismissButton = { TextButton(onClick = vm::discardRecovery) { Text(stringResource(R.string.recovery_discard)) } },
        )
    }

    BackHandler { vm.requestClose() }
    if (vm.askClose) {
        AlertDialog(
            onDismissRequest = { vm.answerClose(null) },
            title = { Text(stringResource(R.string.unsaved_title)) },
            text = { Text(stringResource(R.string.unsaved_body)) },
            confirmButton = {
                TextButton(onClick = {
                    vm.answerClose(true)
                    saveAs.launch(saveName(state.fileName))
                }) { Text(stringResource(R.string.save_as)) }
            },
            dismissButton = {
                Row {
                    TextButton(onClick = { vm.answerClose(null) }) { Text(stringResource(R.string.cancel)) }
                    TextButton(onClick = { vm.answerClose(false) }) { Text(stringResource(R.string.discard)) }
                }
            },
        )
    }

    renaming?.let { old ->
        var name by remember { mutableStateOf(old) }
        AlertDialog(
            onDismissRequest = { renaming = null },
            title = { Text(stringResource(R.string.sheet_rename)) },
            text = { OutlinedTextField(value = name, onValueChange = { name = it.take(31) }, singleLine = true) },
            confirmButton = {
                TextButton(enabled = name.isNotBlank() && name.none { it in "\\/?*[]:" }, onClick = {
                    renaming = null
                    vm.renameSheet(name.trim())
                }) { Text(stringResource(R.string.ok)) }
            },
            dismissButton = { TextButton(onClick = { renaming = null }) { Text(stringResource(R.string.cancel)) } },
        )
    }
    if (confirmDelete) {
        AlertDialog(
            onDismissRequest = { confirmDelete = false },
            title = { Text(stringResource(R.string.sheet_delete)) },
            text = { Text(stringResource(R.string.sheet_delete_body, state.sheetNames.getOrElse(state.sheet) { "" })) },
            confirmButton = {
                TextButton(onClick = {
                    confirmDelete = false
                    vm.deleteSheet()
                }) { Text(stringResource(R.string.delete)) }
            },
            dismissButton = { TextButton(onClick = { confirmDelete = false }) { Text(stringResource(R.string.cancel)) } },
        )
    }

    if (showPdf) {
        PdfSetupSheet(
            vm = vm,
            setup = pdfSetup,
            sheetName = state.sheetNames.getOrElse(state.sheet) { "" },
            onChange = { pdfSetup = it },
            onDismiss = { showPdf = false },
            onSave = {
                showPdf = false
                savePdf.launch(CreateFile.Request(convertedName(state.fileName, "pdf"), "application/pdf"))
            },
        )
    }
}

/** Formatting and rows/columns for the selected cell, all through GridCraft commands. */
@Composable
private fun SheetToolBar(vm: SheetViewModel, look: CellLook?) {
    vm.filterValues?.let { values -> FilterSheet(vm, values) }
    var formatSheet by remember { mutableStateOf(false) }
    if (formatSheet) CellFormatSheet(vm, look, onDismiss = { formatSheet = false })
    var fillMenu by remember { mutableStateOf(false) }
    var borderMenu by remember { mutableStateOf(false) }
    // The tools in groups, as Office's phone app shows them: one row at a time.
    var tab by rememberSaveable { mutableIntStateOf(0) }
    Column(Modifier.fillMaxWidth()) {
        Row(
            Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 8.dp, vertical = 2.dp),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = vm::undo, modifier = Modifier.described(stringResource(R.string.undo))) { Text("↶", fontSize = 18.sp) }
            IconButton(onClick = vm::redo, modifier = Modifier.described(stringResource(R.string.redo))) { Text("↷", fontSize = 18.sp) }
            for ((i, label) in listOf(R.string.sheet_tab_format, R.string.sheet_tab_insert, R.string.sheet_tab_edit, R.string.sheet_tab_data, R.string.sheet_tab_lines).withIndex()) {
                FilterChip(selected = tab == i, onClick = { tab = i }, label = { Text(stringResource(label)) })
            }
        }
        Row(
            Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            when (tab) {
                0 -> {
                    // Each tool is a sign over its name, as in Word's bar, so what it does is plain.
                    Tool("K", R.string.tool_bold, on = look?.bold == true, weight = FontWeight.Bold) { vm.format("home.bold", JSONObject().put("on", look?.bold != true)) }
                    Tool("İ", R.string.tool_italic, on = look?.italic == true, style = FontStyle.Italic) { vm.format("home.italic", JSONObject().put("on", look?.italic != true)) }
                    Tool("A", R.string.tool_underline, on = look?.underline == true, decoration = TextDecoration.Underline) { vm.format("home.underline", JSONObject().put("on", look?.underline != true)) }
                    Box {
                        Tool("🎨", R.string.tool_fill, glyphSize = 18) { fillMenu = true }
                        DropdownMenu(expanded = fillMenu, onDismissRequest = { fillMenu = false }) {
                            for ((label, color) in FILLS) {
                                DropdownMenuItem(text = { Text(stringResource(label)) }, onClick = {
                                    fillMenu = false
                                    vm.format("home.fillColor", JSONObject().put("color", color))
                                })
                            }
                        }
                    }
                    Box {
                        Tool("▦", R.string.tool_borders) { borderMenu = true }
                        DropdownMenu(expanded = borderMenu, onDismissRequest = { borderMenu = false }) {
                            for ((label, preset) in BORDERS) {
                                DropdownMenuItem(text = { Text(stringResource(label)) }, onClick = {
                                    borderMenu = false
                                    vm.format("home.borders", JSONObject().put("preset", preset))
                                })
                            }
                        }
                    }
                    // Merge: the selected cells become one, or a merged cell is split again.
                    val merged = vm.selectionIsMerged()
                    Tool(if (merged) "⊟" else "⊞", if (merged) R.string.tool_unmerge else R.string.tool_merge, on = merged) {
                        vm.format(if (merged) "home.unmergeCells" else "home.mergeCenter")
                    }
                    Tool("⇤", R.string.tool_left, on = look?.align?.toInt() == 1) { vm.format("home.alignLeft") }
                    Tool("≡", R.string.tool_center, on = look?.align?.toInt() == 2) { vm.format("home.alignCenter") }
                    Tool("⇥", R.string.tool_right, on = look?.align?.toInt() == 3) { vm.format("home.alignRight") }
                    Tool("₺", R.string.tool_currency) { vm.format("home.accounting", JSONObject().put("symbol", "₺")) }
                    Tool("%", R.string.tool_percent) { vm.format("home.percent") }
                    Tool(",0→", R.string.tool_more_decimals, glyphSize = 15) { vm.format("home.increaseDecimal") }
                    Tool("←,0", R.string.tool_fewer_decimals, glyphSize = 15) { vm.format("home.decreaseDecimal") }
                    Box {
                        var cfMenu by remember { mutableStateOf(false) }
                        var overDialog by remember { mutableStateOf(false) }
                        Tool("◧", R.string.tool_conditional) { cfMenu = true }
                        DropdownMenu(expanded = cfMenu, onDismissRequest = { cfMenu = false }) {
                            DropdownMenuItem(text = { Text(stringResource(R.string.cf_color_scale)) }, onClick = {
                                cfMenu = false
                                vm.conditionalFormat(JSONObject().put("type", "colorScale").put("colors", org.json.JSONArray(listOf("#F8696B", "#FFEB84", "#63BE7B"))))
                            })
                            DropdownMenuItem(text = { Text(stringResource(R.string.cf_data_bars)) }, onClick = {
                                cfMenu = false
                                vm.conditionalFormat(JSONObject().put("type", "dataBar").put("color", "#638EC6"))
                            })
                            DropdownMenuItem(text = { Text(stringResource(R.string.cf_icons)) }, onClick = {
                                cfMenu = false
                                vm.conditionalFormat(JSONObject().put("type", "iconSet").put("set", "3Arrows"))
                            })
                            DropdownMenuItem(text = { Text(stringResource(R.string.cf_greater)) }, onClick = {
                                cfMenu = false
                                overDialog = true
                            })
                            DropdownMenuItem(text = { Text(stringResource(R.string.cf_duplicates)) }, onClick = {
                                cfMenu = false
                                vm.conditionalFormat(JSONObject().put("type", "duplicate").put("preset", "lightRedFillDarkRedText"))
                            })
                            DropdownMenuItem(text = { Text(stringResource(R.string.cf_clear)) }, onClick = {
                                cfMenu = false
                                vm.clearConditionalFormats()
                            })
                        }
                        if (overDialog) {
                            var value by remember { mutableStateOf("") }
                            AlertDialog(
                                onDismissRequest = { overDialog = false },
                                title = { Text(stringResource(R.string.cf_greater)) },
                                text = {
                                    OutlinedTextField(
                                        value = value, onValueChange = { value = it }, singleLine = true,
                                        label = { Text(stringResource(R.string.cf_value)) },
                                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                                    )
                                },
                                confirmButton = {
                                    TextButton(onClick = {
                                        overDialog = false
                                        // Turkish decimal comma typed: the engine reads a point.
                                        val v = if (vm.turkish) value.trim().replace(".", "").replace(',', '.') else value.trim()
                                        vm.conditionalFormat(JSONObject().put("type", "cellIs").put("operator", "greaterThan").put("value", v).put("preset", "lightRedFillDarkRedText"))
                                    }, enabled = value.isNotBlank()) { Text(stringResource(R.string.apply)) }
                                },
                                dismissButton = { TextButton(onClick = { overDialog = false }) { Text(stringResource(R.string.cancel)) } },
                            )
                        }
                    }
                    Tool("Aa", R.string.tool_format_more, glyphSize = 16) { formatSheet = true }
                }
                1 -> {
                    // Insert: a table over the selection, a chart, a note, a drop-down list.
                    Box {
                        var tableMenu by remember { mutableStateOf(false) }
                        TextButton(onClick = { tableMenu = true }) { Text("▦ " + stringResource(R.string.insert_table_sheet)) }
                        DropdownMenu(expanded = tableMenu, onDismissRequest = { tableMenu = false }) {
                            Text(stringResource(R.string.table_style_hint), style = MaterialTheme.typography.bodySmall, modifier = Modifier.padding(horizontal = 12.dp))
                            for ((style, color) in TABLE_STYLES) {
                                DropdownMenuItem(
                                    text = { Box(Modifier.size(64.dp, 18.dp).background(color, RoundedCornerShape(4.dp))) },
                                    onClick = {
                                        tableMenu = false
                                        vm.format("home.formatAsTable", JSONObject().put("style", style).put("header", true))
                                    },
                                )
                            }
                        }
                    }
                    Box {
                        var chartMenu by remember { mutableStateOf(false) }
                        TextButton(onClick = { chartMenu = true }) { Text("📊 " + stringResource(R.string.chart_insert)) }
                        DropdownMenu(expanded = chartMenu, onDismissRequest = { chartMenu = false }) {
                            for ((type, label) in listOf("column" to R.string.chart_column, "bar" to R.string.chart_bar, "line" to R.string.chart_line, "pie" to R.string.chart_pie, "area" to R.string.chart_area)) {
                                DropdownMenuItem(text = { Text(stringResource(label)) }, onClick = {
                                    chartMenu = false
                                    vm.insertChart(type)
                                })
                            }
                        }
                    }
                    Box {
                        var noteDialog by remember { mutableStateOf(false) }
                        TextButton(onClick = { noteDialog = true }) { Text("🗒 " + stringResource(R.string.sheet_note)) }
                        if (noteDialog) {
                            val existing = vm.sheetState?.let { st -> st.notes.firstOrNull { it.row.toInt() == st.selectedRow && it.col.toInt() == st.selectedCol }?.text }.orEmpty()
                            var text by remember { mutableStateOf(existing) }
                            AlertDialog(
                                onDismissRequest = { noteDialog = false },
                                title = { Text(stringResource(R.string.sheet_note)) },
                                text = { OutlinedTextField(value = text, onValueChange = { text = it }, minLines = 3, modifier = Modifier.fillMaxWidth()) },
                                confirmButton = {
                                    TextButton(onClick = {
                                        noteDialog = false
                                        vm.setNote(text)
                                    }) { Text(stringResource(if (text.isBlank() && existing.isNotEmpty()) R.string.delete else R.string.apply)) }
                                },
                                dismissButton = { TextButton(onClick = { noteDialog = false }) { Text(stringResource(R.string.cancel)) } },
                            )
                        }
                    }
                    Box {
                        var listDialog by remember { mutableStateOf(false) }
                        TextButton(onClick = { listDialog = true }) { Text("☰ " + stringResource(R.string.dropdown_list)) }
                        if (listDialog) {
                            var items by remember { mutableStateOf(vm.sheetState?.choices?.joinToString(", ").orEmpty()) }
                            AlertDialog(
                                onDismissRequest = { listDialog = false },
                                title = { Text(stringResource(R.string.dropdown_list)) },
                                text = {
                                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                                        Text(stringResource(R.string.dropdown_hint), style = MaterialTheme.typography.bodySmall)
                                        OutlinedTextField(value = items, onValueChange = { items = it }, modifier = Modifier.fillMaxWidth())
                                        TextButton(onClick = {
                                            listDialog = false
                                            vm.format("data.validation", JSONObject().put("clear", true))
                                        }) { Text(stringResource(R.string.dropdown_remove)) }
                                    }
                                },
                                confirmButton = {
                                    TextButton(enabled = items.isNotBlank(), onClick = {
                                        listDialog = false
                                        // Excel's list: the choices in quotes, separated by commas.
                                        val list = items.split(',').map { it.trim().replace("\"", "") }.filter { it.isNotEmpty() }.joinToString(",")
                                        vm.format("data.validation", JSONObject().put("type", "list").put("formula1", "\"$list\"").put("dropdown", true))
                                    }) { Text(stringResource(R.string.apply)) }
                                },
                                dismissButton = { TextButton(onClick = { listDialog = false }) { Text(stringResource(R.string.cancel)) } },
                            )
                        }
                    }
                }
                2 -> {
                    val clipboardContext = androidx.compose.ui.platform.LocalContext.current
                    val clipboard = remember { clipboardContext.getSystemService(android.content.ClipboardManager::class.java) }
                    TextButton(onClick = {
                        vm.copySelection(cut = false) { clipboard?.setPrimaryClip(android.content.ClipData.newPlainText("cells", it)) }
                    }) { Text(stringResource(R.string.copy)) }
                    TextButton(onClick = {
                        vm.copySelection(cut = true) { clipboard?.setPrimaryClip(android.content.ClipData.newPlainText("cells", it)) }
                    }) { Text(stringResource(R.string.cut)) }
                    TextButton(onClick = {
                        vm.paste(clipboard?.primaryClip?.takeIf { it.itemCount > 0 }?.getItemAt(0)?.coerceToText(clipboardContext)?.toString())
                    }) { Text(stringResource(R.string.paste)) }
                    TextButton(onClick = vm::clearSelection) { Text(stringResource(R.string.sheet_clear)) }
                    TextButton(onClick = vm::fillDown) { Text(stringResource(R.string.sheet_fill_down)) }
                }
                3 -> {
                    Box {
                        var sumMenu by remember { mutableStateOf(false) }
                        TextButton(onClick = { sumMenu = true }) { Text("Σ " + stringResource(R.string.autosum)) }
                        DropdownMenu(expanded = sumMenu, onDismissRequest = { sumMenu = false }) {
                            for ((fn, label) in listOf("SUM" to R.string.autosum_sum, "AVERAGE" to R.string.autosum_average, "COUNT" to R.string.autosum_count, "MAX" to R.string.autosum_max, "MIN" to R.string.autosum_min)) {
                                DropdownMenuItem(text = { Text(stringResource(label)) }, onClick = {
                                    sumMenu = false
                                    vm.autoSum(fn)
                                })
                            }
                        }
                    }
                    TextButton(onClick = { vm.sort(true) }) { Text("A→Z") }
                    TextButton(onClick = { vm.sort(false) }) { Text("Z→A") }
                    TextButton(onClick = vm::openFilter) { Text((if (vm.hasFilter) "▼● " else "▼ ") + stringResource(R.string.filter)) }
                    TextButton(onClick = { vm.format("data.removeDuplicates") }) { Text(stringResource(R.string.remove_duplicates)) }
                }
                else -> {
                    TextButton(onClick = { vm.lines(rows = true, insert = true) }) { Text(stringResource(R.string.row_insert)) }
                    TextButton(onClick = { vm.lines(rows = true, insert = false) }) { Text(stringResource(R.string.row_delete)) }
                    TextButton(onClick = { vm.lines(rows = false, insert = true) }) { Text(stringResource(R.string.col_insert)) }
                    TextButton(onClick = { vm.lines(rows = false, insert = false) }) { Text(stringResource(R.string.col_delete)) }
                    TextButton(onClick = { vm.hideLines(rows = true) }) { Text(stringResource(R.string.row_hide)) }
                    TextButton(onClick = { vm.hideLines(rows = false) }) { Text(stringResource(R.string.col_hide)) }
                    TextButton(onClick = vm::unhideAll) { Text(stringResource(R.string.unhide_all)) }
                    TextButton(onClick = vm::autofitColumns) { Text(stringResource(R.string.col_autofit)) }
                }
            }
        }
    }
}

/** Border presets offered (GridCraft's home.borders). */
private val BORDERS = listOf(
    R.string.border_all to "all",
    R.string.border_outside to "outside",
    R.string.border_thick_outside to "thickOutside",
    R.string.border_bottom to "bottom",
    R.string.border_none to "none",
)

/** Fill colours offered: none, and the light shades people mark rows with. */
private val FILLS = listOf(
    R.string.fill_none to "none",
    R.string.fill_yellow to "#FFF2CC",
    R.string.fill_green to "#E2EFDA",
    R.string.fill_blue to "#DDEBF7",
    R.string.fill_red to "#FCE4D6",
    R.string.fill_grey to "#EDEDED",
)

@Composable
private fun FormulaBar(
    state: SheetUiState,
    value: TextFieldValue,
    onValue: (TextFieldValue) -> Unit,
    focused: Boolean,
    onFocus: (Boolean) -> Unit,
    docs: List<FunctionDoc>,
    turkish: Boolean,
    onAllFormulas: () -> Unit,
    onCommit: (String) -> Unit,
) {
    val names = remember(docs) { docs.map { it.name } }
    Column {
        Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(
                rangeName(state),
                Modifier.width(56.dp),
                fontFamily = FontFamily.Monospace,
                style = MaterialTheme.typography.labelLarge,
            )
            TextButton(onClick = onAllFormulas, contentPadding = PaddingValues(horizontal = 6.dp)) {
                Text("ƒx", fontStyle = FontStyle.Italic, fontWeight = FontWeight.Bold)
            }
            OutlinedTextField(
                value = value,
                onValueChange = onValue,
                modifier = Modifier.weight(1f).onFocusChanged { onFocus(it.isFocused) },
                singleLine = true,
                placeholder = { Text(stringResource(R.string.formula_hint)) },
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                keyboardActions = KeyboardActions(onDone = { onCommit(value.text) }),
            )
        }
        // While typing: what the function around the cursor takes, names that fit what is typed
        // (with what the first one does), else the keys a formula needs.
        if (focused) {
            val offered = suggestions(value, names)
            val shownDoc = (offered.firstOrNull() ?: functionAtCursor(value))?.let { n -> docFor(n, docs) { it.name } }
            if (shownDoc != null) {
                Column(Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 2.dp)) {
                    Text(shownDoc.signature, fontFamily = FontFamily.Monospace, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.primary)
                    if (shownDoc.description.isNotEmpty()) {
                        Text(shownDoc.description, style = MaterialTheme.typography.bodySmall, maxLines = 2, overflow = TextOverflow.Ellipsis)
                    }
                }
            } else if (pointing(value)) {
                Text(
                    stringResource(R.string.formula_point_hint),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(horizontal = 12.dp),
                )
            }
            Row(
                Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 8.dp),
                horizontalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                if (offered.isNotEmpty()) {
                    for (name in offered) {
                        AssistChip(onClick = { onValue(applySuggestion(value, name)) }, label = { Text(name) })
                    }
                } else {
                    val separator = if (turkish) ";" else ","
                    for (key in listOf("=", "+", "-", "*", "/", "(", ")", separator, ":", "$", "%", "&", "\"\"")) {
                        AssistChip(onClick = { onValue(typeAt(value, key)) }, label = { Text(key, fontFamily = FontFamily.Monospace) })
                    }
                }
            }
        }
    }
}

/** Every function, searchable and grouped by category; a tap puts it in the formula. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun FormulaHelpSheet(docs: List<FunctionDoc>, onDismiss: () -> Unit, onPick: (String) -> Unit) {
    var query by remember { mutableStateOf("") }
    val folded = query.trim().searchFold()
    val shown = remember(docs, folded) {
        if (folded.isEmpty()) docs else docs.filter { it.name.searchFold().contains(folded) || it.description.searchFold().contains(folded) }
    }
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(Modifier.padding(horizontal = 16.dp)) {
            Text(stringResource(R.string.all_formulas), style = MaterialTheme.typography.titleLarge)
            OutlinedTextField(
                value = query,
                onValueChange = { query = it },
                placeholder = { Text(stringResource(R.string.formula_search)) },
                singleLine = true,
                modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp),
            )
        }
        LazyColumn(Modifier.fillMaxWidth().heightIn(max = 560.dp)) {
            items(shown, key = { it.name }) { d ->
                ListItem(
                    overlineContent = if (d.category.isNotEmpty()) ({ Text(d.category) }) else null,
                    headlineContent = { Text(d.signature, fontFamily = FontFamily.Monospace, style = MaterialTheme.typography.bodyMedium) },
                    supportingContent = if (d.description.isNotEmpty()) ({ Text(d.description) }) else null,
                    trailingContent = { TextButton(onClick = { onPick(d.name) }) { Text(stringResource(R.string.formula_insert)) } },
                    modifier = Modifier.clickable { onPick(d.name) },
                )
                HorizontalDivider()
            }
        }
    }
}

/** Saving always writes XLSX, so CSV/XLS imports get a .xlsx name. */
private fun saveName(fileName: String): String {
    val stem = fileName.substringBeforeLast('.', fileName)
    return if (fileName.endsWith(".xlsx", ignoreCase = true)) fileName else "$stem.xlsx"
}

/** Page Setup for a spreadsheet PDF: which sheets or range, paper, orientation, margins, scaling. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun PdfSetupSheet(vm: SheetViewModel, setup: PdfSetup, sheetName: String, onChange: (PdfSetup) -> Unit, onDismiss: () -> Unit, onSave: () -> Unit) {
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(Modifier.padding(horizontal = 24.dp).verticalScroll(rememberScrollState())) {
            Text(stringResource(R.string.pdf_setup), style = MaterialTheme.typography.titleLarge)
            Spacer(Modifier.height(8.dp))
            // What the PDF will look like, redrawn a moment after each change.
            var preview by remember { mutableStateOf<PdfPreview?>(null) }
            var drawing by remember { mutableStateOf(true) }
            val thumbPx = with(androidx.compose.ui.platform.LocalDensity.current) { 120.dp.roundToPx() }
            LaunchedEffect(setup) {
                drawing = true
                kotlinx.coroutines.delay(350)
                preview = vm.previewPdf(setup, thumbPx * 2)
                drawing = false
            }
            Row(Modifier.horizontalScroll(rememberScrollState()).height(170.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                preview?.images?.forEach { img ->
                    Image(
                        img.asImageBitmap(),
                        contentDescription = null,
                        modifier = Modifier.width(120.dp).fillMaxHeight()
                            .border(1.dp, MaterialTheme.colorScheme.outlineVariant)
                            .background(Color.White),
                        contentScale = ContentScale.Fit,
                    )
                }
            }
            Text(
                when {
                    drawing && preview == null -> stringResource(R.string.pdf_preview_drawing)
                    preview == null -> ""
                    else -> pluralStringResource(R.plurals.pdf_preview_pages, preview!!.pages, preview!!.pages)
                },
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 4.dp, bottom = 4.dp),
            )
            Choice(stringResource(R.string.pdf_what)) {
                Chip(stringResource(R.string.pdf_all_sheets), setup.allSheets) { onChange(setup.copy(allSheets = true)) }
                Chip(stringResource(R.string.pdf_this_sheet, sheetName), !setup.allSheets) { onChange(setup.copy(allSheets = false)) }
            }
            if (!setup.allSheets) {
                OutlinedTextField(
                    value = setup.range,
                    onValueChange = { onChange(setup.copy(range = it.uppercase(Locale.ROOT))) },
                    label = { Text(stringResource(R.string.pdf_print_area)) },
                    placeholder = { Text("A1:F40") },
                    supportingText = { Text(stringResource(R.string.pdf_print_area_hint)) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
            Choice(stringResource(R.string.pdf_orientation)) {
                Chip(stringResource(R.string.pdf_portrait), !setup.landscape) { onChange(setup.copy(landscape = false)) }
                Chip(stringResource(R.string.pdf_landscape), setup.landscape) { onChange(setup.copy(landscape = true)) }
            }
            Choice(stringResource(R.string.pdf_paper)) {
                for (p in listOf("A4", "A3", "A5", "Letter")) Chip(p, setup.paper == p) { onChange(setup.copy(paper = p)) }
            }
            Choice(stringResource(R.string.pdf_margins)) {
                Chip(stringResource(R.string.pdf_margins_normal), setup.margins == "normal") { onChange(setup.copy(margins = "normal")) }
                Chip(stringResource(R.string.pdf_margins_wide), setup.margins == "wide") { onChange(setup.copy(margins = "wide")) }
                Chip(stringResource(R.string.pdf_margins_narrow), setup.margins == "narrow") { onChange(setup.copy(margins = "narrow")) }
            }
            Choice(stringResource(R.string.pdf_scaling)) {
                Chip(stringResource(R.string.pdf_fit_none), setup.fit == "none") { onChange(setup.copy(fit = "none")) }
                Chip(stringResource(R.string.pdf_fit_columns), setup.fit == "columns") { onChange(setup.copy(fit = "columns")) }
                Chip(stringResource(R.string.pdf_fit_page), setup.fit == "page") { onChange(setup.copy(fit = "page")) }
            }
            SetupSwitch(stringResource(R.string.pdf_repeat_header), setup.repeatHeader) { onChange(setup.copy(repeatHeader = it)) }
            SetupSwitch(stringResource(R.string.pdf_page_numbers), setup.pageNumbers) { onChange(setup.copy(pageNumbers = it)) }
            SetupSwitch(stringResource(R.string.pdf_gridlines), setup.gridlines) { onChange(setup.copy(gridlines = it)) }
            SetupSwitch(stringResource(R.string.pdf_headings), setup.headings) { onChange(setup.copy(headings = it)) }
            SetupSwitch(stringResource(R.string.pdf_center_h), setup.centerH) { onChange(setup.copy(centerH = it)) }
            Button(onClick = onSave, modifier = Modifier.fillMaxWidth().padding(vertical = 12.dp)) {
                Text(stringResource(R.string.pdf_save))
            }
        }
    }
}

@Composable
private fun SetupSwitch(title: String, on: Boolean, onChange: (Boolean) -> Unit) {
    ListItem(
        headlineContent = { Text(title) },
        trailingContent = { Switch(on, onCheckedChange = null) },
        modifier = Modifier.toggleable(value = on, role = Role.Switch, onValueChange = onChange),
    )
}

@Composable
private fun Choice(title: String, chips: @Composable () -> Unit) {
    Text(title, style = MaterialTheme.typography.labelLarge, modifier = Modifier.padding(top = 8.dp))
    Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) { chips() }
}

@Composable
private fun Chip(label: String, selected: Boolean, onClick: () -> Unit) {
    FilterChip(selected = selected, onClick = onClick, label = { Text(label) })
}

/** The field laid over a cell being typed in: opens the keyboard at once, Enter finishes. */
@Composable
private fun CellEditor(value: TextFieldValue, onValue: (TextFieldValue) -> Unit, onFocus: (Boolean) -> Unit, onDone: () -> Unit, modifier: Modifier) {
    val focus = remember { FocusRequester() }
    LaunchedEffect(Unit) { focus.requestFocus() }
    BasicTextField(
        value = value,
        onValueChange = onValue,
        singleLine = true,
        textStyle = MaterialTheme.typography.bodyLarge.copy(color = Color.Black),
        cursorBrush = SolidColor(MaterialTheme.colorScheme.primary),
        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
        keyboardActions = KeyboardActions(onDone = { onDone() }),
        modifier = modifier
            .shadow(6.dp, RoundedCornerShape(4.dp))
            .background(Color.White, RoundedCornerShape(4.dp))
            .border(2.dp, MaterialTheme.colorScheme.primary, RoundedCornerShape(4.dp))
            .padding(horizontal = 6.dp, vertical = 8.dp)
            .focusRequester(focus)
            .onFocusChanged { onFocus(it.isFocused) },
    )
}

/** Number formats offered: label and Excel format code. */
private val NUMBER_FORMATS = listOf(
    R.string.nf_general to "General",
    R.string.nf_number to "#,##0.00",
    R.string.nf_currency to "#,##0.00 \"₺\"",
    R.string.nf_percent to "0%",
    R.string.nf_date to "dd.mm.yyyy",
    R.string.nf_time to "hh:mm",
    R.string.nf_text to "@",
)

/** Table styles offered, with a colour that shows each one. */
private val TABLE_STYLES = listOf(
    "TableStyleMedium2" to Color(0xFF4472C4),
    "TableStyleMedium4" to Color(0xFF70AD47),
    "TableStyleMedium3" to Color(0xFFED7D31),
    "TableStyleMedium7" to Color(0xFFFFC000),
    "TableStyleLight9" to Color(0xFF5B9BD5),
    "TableStyleMedium1" to Color(0xFF404040),
)

private val FONT_COLORS = listOf("#000000", "#C00000", "#FF0000", "#ED7D31", "#00B050", "#0070C0", "#7030A0", "#808080")

/** Everything about how the selected cells look and read, in one sheet. */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
private fun CellFormatSheet(vm: SheetViewModel, look: CellLook?, onDismiss: () -> Unit) {
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(Modifier.padding(horizontal = 20.dp).verticalScroll(rememberScrollState()).padding(bottom = 24.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Text(stringResource(R.string.cell_format_title), style = MaterialTheme.typography.titleLarge)
            Text(stringResource(R.string.number_format), style = MaterialTheme.typography.labelLarge)
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                for ((label, code) in NUMBER_FORMATS) {
                    AssistChip(onClick = { vm.format("home.numberFormat", JSONObject().put("code", code)) }, label = { Text(stringResource(label)) })
                }
            }
            Text(stringResource(R.string.font_size_label), style = MaterialTheme.typography.labelLarge)
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                OutlinedButton(onClick = { vm.format("home.decreaseFontSize") }, modifier = Modifier.described(stringResource(R.string.a11y_text_smaller))) { Text("A−") }
                OutlinedButton(onClick = { vm.format("home.increaseFontSize") }, modifier = Modifier.described(stringResource(R.string.a11y_text_larger))) { Text("A+") }
                for (size in listOf(10, 11, 12, 14, 16, 20)) {
                    TextButton(onClick = { vm.format("home.fontSize", JSONObject().put("size", size)) }, contentPadding = PaddingValues(6.dp)) { Text(size.toString()) }
                }
            }
            Text(stringResource(R.string.font_color), style = MaterialTheme.typography.labelLarge)
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                for (hex in FONT_COLORS) {
                    Box(
                        Modifier.size(32.dp).clip(CircleShape).background(Color(android.graphics.Color.parseColor(hex)))
                            .border(1.dp, MaterialTheme.colorScheme.outline, CircleShape)
                            .clickable(role = Role.Button) { vm.format("home.fontColor", JSONObject().put("color", hex)) }
                            .described(stringResource(R.string.a11y_color_n, FONT_COLORS.indexOf(hex) + 1)),
                    )
                }
            }
            ListItem(
                headlineContent = { Text(stringResource(R.string.wrap_text)) },
                trailingContent = { Switch(checked = look?.wrap == true, onCheckedChange = null) },
                modifier = Modifier.toggleable(value = look?.wrap == true, role = Role.Switch) { vm.format("home.wrapText", JSONObject().put("on", it)) },
            )
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                OutlinedButton(onClick = { vm.format("home.mergeCenter") }) { Text(stringResource(R.string.merge_center)) }
                OutlinedButton(onClick = { vm.format("home.unmergeCells") }) { Text(stringResource(R.string.unmerge)) }
                OutlinedButton(onClick = { vm.format("home.autofitColumnWidth") }) { Text(stringResource(R.string.autofit)) }
            }
            Text(stringResource(R.string.format_as_table), style = MaterialTheme.typography.labelLarge)
            Text(stringResource(R.string.table_style_hint), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                for ((style, color) in TABLE_STYLES) {
                    // A tiny table: a coloured header over banded rows.
                    Column(
                        Modifier.size(44.dp, 40.dp).clip(RoundedCornerShape(6.dp)).border(1.dp, MaterialTheme.colorScheme.outlineVariant, RoundedCornerShape(6.dp))
                            .clickable(role = Role.Button) {
                                vm.format("home.formatAsTable", JSONObject().put("style", style))
                                onDismiss()
                            }
                            .described(stringResource(R.string.a11y_table_style_n, TABLE_STYLES.indexOfFirst { it.first == style } + 1)),
                    ) {
                        Box(Modifier.fillMaxWidth().weight(1f).background(color))
                        Box(Modifier.fillMaxWidth().weight(1f).background(color.copy(alpha = 0.25f)))
                        Box(Modifier.fillMaxWidth().weight(1f).background(Color.White))
                        Box(Modifier.fillMaxWidth().weight(1f).background(color.copy(alpha = 0.25f)))
                    }
                }
            }
        }
    }
}

/** Excel's column filter: tick the values whose rows stay in view. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun FilterSheet(vm: SheetViewModel, values: List<org.mobileofficefree.engine.SheetFilterValue>) {
    val ticked = remember(values) { mutableStateMapOf<String, Boolean>().apply { values.forEach { put(it.value, it.selected) } } }
    var query by remember { mutableStateOf("") }
    val shown = values.filter { query.isBlank() || it.label.contains(query, ignoreCase = true) }
    ModalBottomSheet(onDismissRequest = vm::closeFilter) {
        Column(Modifier.padding(horizontal = 16.dp)) {
            Text(stringResource(R.string.filter_title, columnName(vm.filterColumn)), style = MaterialTheme.typography.titleLarge)
            if (values.isEmpty()) {
                Text(stringResource(R.string.filter_nothing), modifier = Modifier.padding(vertical = 16.dp))
            } else {
                OutlinedTextField(
                    value = query, onValueChange = { query = it }, singleLine = true,
                    placeholder = { Text(stringResource(R.string.search)) },
                    modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp),
                )
                Row {
                    TextButton(onClick = { shown.forEach { ticked[it.value] = true } }) { Text(stringResource(R.string.filter_all)) }
                    TextButton(onClick = { shown.forEach { ticked[it.value] = false } }) { Text(stringResource(R.string.filter_none)) }
                }
                LazyColumn(Modifier.heightIn(max = 360.dp)) {
                    items(shown, key = { it.value }) { v ->
                        Row(
                            Modifier.fillMaxWidth().toggleable(value = ticked[v.value] == true, role = Role.Checkbox) { ticked[v.value] = it },
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Checkbox(checked = ticked[v.value] == true, onCheckedChange = null)
                            Text(v.label.ifEmpty { stringResource(R.string.filter_blanks) }, modifier = Modifier.weight(1f), maxLines = 1, overflow = TextOverflow.Ellipsis)
                            Text("${v.count}", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                    }
                }
            }
            Row(Modifier.fillMaxWidth().padding(vertical = 12.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                if (vm.hasFilter) OutlinedButton(onClick = vm::removeFilter) { Text(stringResource(R.string.filter_remove)) }
                Spacer(Modifier.weight(1f))
                Button(
                    onClick = { vm.applyFilter(values.filter { ticked[it.value] == true }.map { it.value }) },
                    enabled = values.isNotEmpty() && ticked.values.any { it },
                ) { Text(stringResource(R.string.apply)) }
            }
        }
    }
}

/** Find and replace across the workbook's sheets, ignoring case the Turkish way. */
@Composable
private fun FindBar(vm: SheetViewModel, onClose: () -> Unit) {
    var query by remember { mutableStateOf("") }
    var with by remember { mutableStateOf("") }
    var replacing by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            OutlinedTextField(
                value = query,
                onValueChange = { query = it },
                singleLine = true,
                placeholder = { Text(stringResource(R.string.search)) },
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
                keyboardActions = KeyboardActions(onSearch = { vm.find(query) }),
                modifier = Modifier.weight(1f),
            )
            Text(
                when {
                    vm.findHits.isNotEmpty() -> "${vm.findIndex + 1}/${vm.findHits.size}"
                    else -> ""
                },
                style = MaterialTheme.typography.labelMedium,
                modifier = Modifier.padding(horizontal = 6.dp),
            )
            TextButton(onClick = { if (vm.findHits.isEmpty()) vm.find(query) else vm.nextHit(-1) }, enabled = query.isNotBlank(), modifier = Modifier.described(stringResource(R.string.a11y_previous_match))) { Text("▲") }
            TextButton(onClick = { if (vm.findHits.isEmpty()) vm.find(query) else vm.nextHit(1) }, enabled = query.isNotBlank(), modifier = Modifier.described(stringResource(R.string.a11y_next_match))) { Text("▼") }
            TextButton(onClick = onClose, modifier = Modifier.described(stringResource(R.string.close))) { Text("✕") }
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            if (replacing) {
                OutlinedTextField(
                    value = with,
                    onValueChange = { with = it },
                    singleLine = true,
                    placeholder = { Text(stringResource(R.string.replace_with)) },
                    modifier = Modifier.weight(1f),
                )
                TextButton(onClick = { vm.replaceAll(query, with) }, enabled = query.isNotBlank()) { Text(stringResource(R.string.replace_all)) }
            } else {
                TextButton(onClick = { replacing = true }) { Text(stringResource(R.string.replace)) }
            }
        }
    }
    LaunchedEffect(query) {
        // A new word is looked for afresh.
        vm.endFind()
    }
}

/** What a tapped chart offers: move it, change its type or title, delete it. */
@Composable
private fun ChartMenu(vm: SheetViewModel) {
    var title by remember { mutableStateOf("") }
    val moveHint = stringResource(R.string.chart_move_hint)
    AlertDialog(
        onDismissRequest = { vm.pickChart(null) },
        title = { Text(stringResource(R.string.chart_menu_title)) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(onClick = {
                    vm.movingChart = true
                    vm.hint(moveHint)
                }, modifier = Modifier.fillMaxWidth()) { Text("✥ " + stringResource(R.string.chart_move)) }
                Text(stringResource(R.string.chart_type), style = MaterialTheme.typography.labelLarge)
                Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    for ((type, label) in listOf("column" to R.string.chart_column, "bar" to R.string.chart_bar, "line" to R.string.chart_line, "pie" to R.string.chart_pie, "area" to R.string.chart_area)) {
                        AssistChip(onClick = { vm.changeChart(JSONObject().put("type", type)) }, label = { Text(stringResource(label)) })
                    }
                }
                OutlinedTextField(
                    value = title,
                    onValueChange = { title = it },
                    singleLine = true,
                    label = { Text(stringResource(R.string.chart_title)) },
                    trailingIcon = {
                        TextButton(onClick = { vm.changeChart(JSONObject().put("title", title.trim())) }, enabled = title.isNotBlank()) { Text(stringResource(R.string.apply)) }
                    },
                )
                TextButton(onClick = { vm.changeChart(JSONObject(), delete = true) }) {
                    Text(stringResource(R.string.chart_delete), color = MaterialTheme.colorScheme.error)
                }
            }
        },
        confirmButton = { TextButton(onClick = { vm.pickChart(null) }) { Text(stringResource(R.string.close)) } },
    )
}
