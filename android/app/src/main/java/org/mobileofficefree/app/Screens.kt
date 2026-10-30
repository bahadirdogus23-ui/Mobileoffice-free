package org.mobileofficefree.app

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
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
import androidx.compose.material3.Button
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
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import java.util.Locale

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
    ) { uri -> if (uri != null) vm.saveTo(uri) }
    var pdfSetup by remember { mutableStateOf(PdfSetup()) }
    var showPdf by remember { mutableStateOf(false) }
    val savePdf = rememberLauncherForActivityResult(CreateFile()) { uri -> if (uri != null) vm.exportPdfTo(uri, pdfSetup) }

    Column(Modifier.fillMaxSize().navigationBarsPadding()) {
        TopAppBar(
            title = { Text(state.fileName, maxLines = 1) },
            navigationIcon = {
                IconButton(onClick = vm::close) {
                    Icon(painterResource(R.drawable.ic_arrow_back), contentDescription = stringResource(R.string.back))
                }
            },
            actions = {
                TextButton(onClick = { showPdf = true }) { Text("PDF") }
                TextButton(onClick = { saveAs.launch(saveName(state.fileName)) }) {
                    Text(stringResource(R.string.save_as))
                }
            },
        )
        FormulaBar(state, onCommit = vm::commitCell)
        HorizontalDivider()
        SheetGrid(
            sheet = state.sheet,
            usedRows = state.usedRows,
            usedCols = state.usedCols,
            selectedRow = state.selectedRow,
            selectedCol = state.selectedCol,
            cellText = vm::cellText,
            onSelect = vm::selectCell,
            modifier = Modifier.weight(1f),
        )
        if (state.sheetNames.size > 1) {
            PrimaryScrollableTabRow(selectedTabIndex = state.sheet) {
                state.sheetNames.forEachIndexed { i, name ->
                    Tab(selected = i == state.sheet, onClick = { vm.selectSheet(i) }, text = { Text(name) })
                }
            }
        }
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

@Composable
private fun FormulaBar(state: SheetUiState, onCommit: (String) -> Unit) {
    // Reset the field whenever the selection or the cell's content changes.
    var text by rememberSaveable(state.sheet, state.selectedRow, state.selectedCol, state.editText) {
        mutableStateOf(state.editText)
    }
    Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(
            cellName(state.selectedRow, state.selectedCol),
            Modifier.width(64.dp),
            fontFamily = FontFamily.Monospace,
            style = MaterialTheme.typography.labelLarge,
        )
        OutlinedTextField(
            value = text,
            onValueChange = { text = it },
            modifier = Modifier.weight(1f),
            singleLine = true,
            placeholder = { Text(stringResource(R.string.formula_hint)) },
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
            keyboardActions = KeyboardActions(onDone = { onCommit(text) }),
        )
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
