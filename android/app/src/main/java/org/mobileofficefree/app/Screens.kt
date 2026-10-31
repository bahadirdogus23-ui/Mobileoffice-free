package org.mobileofficefree.app

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
import androidx.compose.material3.FilledIconToggleButton
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
    var showPdf by remember { mutableStateOf(false) }
    var sheetMenu by remember { mutableStateOf(false) }
    var renaming by remember { mutableStateOf<String?>(null) }
    var confirmDelete by remember { mutableStateOf(false) }
    val savePdf = rememberLauncherForActivityResult(CreateFile()) { uri -> if (uri != null) vm.exportPdfTo(uri, pdfSetup) }

    Column(Modifier.fillMaxSize().navigationBarsPadding()) {
        TopAppBar(
            title = { Text(state.fileName, maxLines = 1) },
            navigationIcon = {
                IconButton(onClick = vm::requestClose) {
                    Icon(painterResource(R.drawable.ic_arrow_back), contentDescription = stringResource(R.string.back))
                }
            },
            actions = {
                if (vm.canSaveInPlace && vm.hasUnsaved) {
                    TextButton(onClick = vm::saveInPlace) { Text(stringResource(R.string.save)) }
                }
                TextButton(onClick = { showPdf = true }) { Text("PDF") }
                TextButton(onClick = { saveAs.launch(saveName(state.fileName)) }) {
                    Text(stringResource(R.string.save_as))
                }
            },
        )
        FormulaBar(state, vm.functionNames, vm.turkish, onCommit = vm::commitCell)
        SheetToolBar(vm, vm.cellLook(state.sheet, state.selectedRow, state.selectedCol))
        HorizontalDivider()
        SheetGrid(
            sheet = state.sheet,
            usedRows = state.usedRows,
            usedCols = state.usedCols,
            selectedRow = state.selectedRow,
            selectedCol = state.selectedCol,
            cellText = vm::cellText,
            cellLook = vm::cellLook,
            columnAxis = state.columns,
            rowAxis = state.rows,
            frozenRows = state.frozenRows,
            frozenCols = state.frozenCols,
            onSelect = vm::selectCell,
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
    var fillMenu by remember { mutableStateOf(false) }
    Row(
        Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        FilledIconToggleButton(checked = look?.bold == true, onCheckedChange = { vm.format("home.bold", JSONObject().put("on", it)) }) {
            Text("K", fontWeight = FontWeight.Bold)
        }
        FilledIconToggleButton(checked = look?.italic == true, onCheckedChange = { vm.format("home.italic", JSONObject().put("on", it)) }) {
            Text("İ", fontStyle = FontStyle.Italic)
        }
        FilledIconToggleButton(checked = look?.underline == true, onCheckedChange = { vm.format("home.underline", JSONObject().put("on", it)) }) {
            Text("A", textDecoration = TextDecoration.Underline)
        }
        Box {
            TextButton(onClick = { fillMenu = true }) { Text("🎨") }
            DropdownMenu(expanded = fillMenu, onDismissRequest = { fillMenu = false }) {
                for ((label, color) in FILLS) {
                    DropdownMenuItem(text = { Text(stringResource(label)) }, onClick = {
                        fillMenu = false
                        vm.format("home.fillColor", JSONObject().put("color", color))
                    })
                }
            }
        }
        TextButton(onClick = { vm.format("home.alignLeft") }) { Text("⇤") }
        TextButton(onClick = { vm.format("home.alignCenter") }) { Text("≡") }
        TextButton(onClick = { vm.format("home.alignRight") }) { Text("⇥") }
        TextButton(onClick = { vm.format("home.accounting", JSONObject().put("symbol", "₺")) }) { Text("₺") }
        TextButton(onClick = { vm.format("home.percent") }) { Text("%") }
        TextButton(onClick = { vm.sort(true) }) { Text("A→Z") }
        TextButton(onClick = { vm.sort(false) }) { Text("Z→A") }
        TextButton(onClick = { vm.format("home.increaseDecimal") }) { Text(",0→") }
        TextButton(onClick = { vm.format("home.decreaseDecimal") }) { Text("←,0") }
        TextButton(onClick = { vm.lines(rows = true, insert = true) }) { Text(stringResource(R.string.row_insert)) }
        TextButton(onClick = { vm.lines(rows = true, insert = false) }) { Text(stringResource(R.string.row_delete)) }
        TextButton(onClick = { vm.lines(rows = false, insert = true) }) { Text(stringResource(R.string.col_insert)) }
        TextButton(onClick = { vm.lines(rows = false, insert = false) }) { Text(stringResource(R.string.col_delete)) }
        TextButton(onClick = vm::undo) { Text(stringResource(R.string.undo)) }
        TextButton(onClick = vm::redo) { Text(stringResource(R.string.redo)) }
    }
}

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
private fun FormulaBar(state: SheetUiState, names: List<String>, turkish: Boolean, onCommit: (String) -> Unit) {
    // Reset the field whenever the selection or the cell's content changes.
    var value by remember(state.sheet, state.selectedRow, state.selectedCol, state.editText) {
        mutableStateOf(TextFieldValue(state.editText, TextRange(state.editText.length)))
    }
    var focused by remember { mutableStateOf(false) }
    Column {
        Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(
                cellName(state.selectedRow, state.selectedCol),
                Modifier.width(64.dp),
                fontFamily = FontFamily.Monospace,
                style = MaterialTheme.typography.labelLarge,
            )
            OutlinedTextField(
                value = value,
                onValueChange = { value = it },
                modifier = Modifier.weight(1f).onFocusChanged { focused = it.isFocused },
                singleLine = true,
                placeholder = { Text(stringResource(R.string.formula_hint)) },
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                keyboardActions = KeyboardActions(onDone = { onCommit(value.text) }),
            )
        }
        // While typing: function names that fit what is typed, else the keys a formula needs.
        if (focused) {
            val offered = suggestions(value, names)
            Row(
                Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 8.dp),
                horizontalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                if (offered.isNotEmpty()) {
                    for (name in offered) {
                        AssistChip(onClick = { value = applySuggestion(value, name) }, label = { Text(name) })
                    }
                } else {
                    val separator = if (turkish) ";" else ","
                    for (key in listOf("=", "+", "-", "*", "/", "(", ")", separator, ":", "$", "%", "&", "\"\"")) {
                        AssistChip(onClick = { value = typeAt(value, key) }, label = { Text(key, fontFamily = FontFamily.Monospace) })
                    }
                }
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
private fun PdfSetupSheet(setup: PdfSetup, sheetName: String, onChange: (PdfSetup) -> Unit, onDismiss: () -> Unit, onSave: () -> Unit) {
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(Modifier.padding(horizontal = 24.dp).verticalScroll(rememberScrollState())) {
            Text(stringResource(R.string.pdf_setup), style = MaterialTheme.typography.titleLarge)
            Spacer(Modifier.height(12.dp))
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
            ListItem(
                headlineContent = { Text(stringResource(R.string.pdf_fit)) },
                supportingContent = { Text(stringResource(R.string.pdf_fit_hint)) },
                trailingContent = { Switch(setup.fitToPage, { onChange(setup.copy(fitToPage = it)) }) },
            )
            ListItem(
                headlineContent = { Text(stringResource(R.string.pdf_gridlines)) },
                trailingContent = { Switch(setup.gridlines, { onChange(setup.copy(gridlines = it)) }) },
            )
            Button(onClick = onSave, modifier = Modifier.fillMaxWidth().padding(vertical = 12.dp)) {
                Text(stringResource(R.string.pdf_save))
            }
        }
    }
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
