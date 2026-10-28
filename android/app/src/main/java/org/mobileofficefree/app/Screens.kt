package org.mobileofficefree.app

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.PrimaryScrollableTabRow
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
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
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp

/** Spreadsheet types the picker offers (the engine also sniffs content, so a wrong type still opens). */
private val SPREADSHEET_TYPES = arrayOf(
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "application/vnd.ms-excel.sheet.macroEnabled.12",
    "application/vnd.ms-excel",
    "application/vnd.ms-excel.sheet.binary.macroEnabled.12",
    "application/vnd.oasis.opendocument.spreadsheet",
    "text/csv",
    "text/comma-separated-values",
    "text/tab-separated-values",
)

@Composable
fun AppRoot(vm: SheetViewModel) {
    val snackbar = remember { SnackbarHostState() }
    LaunchedEffect(vm.message) {
        vm.message?.let {
            snackbar.showSnackbar(it)
            vm.consumeMessage()
        }
    }
    Scaffold(snackbarHost = { SnackbarHost(snackbar) }) { padding ->
        Box(Modifier.padding(padding).fillMaxSize()) {
            when (val s = vm.screen) {
                Screen.Home -> HomeScreen(vm)
                Screen.Loading -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        CircularProgressIndicator()
                        Spacer(Modifier.height(12.dp))
                        Text(stringResource(R.string.loading))
                    }
                }
                is Screen.Sheet -> SheetScreen(vm, s.state)
            }
        }
    }
    if (vm.importNotes.isNotEmpty()) {
        AlertDialog(
            onDismissRequest = vm::dismissImportNotes,
            confirmButton = { TextButton(onClick = vm::dismissImportNotes) { Text(stringResource(R.string.ok)) } },
            title = { Text(stringResource(R.string.import_notes)) },
            text = { Text(vm.importNotes.take(5).joinToString("\n\n")) },
        )
    }
}

@Composable
private fun HomeScreen(vm: SheetViewModel) {
    val pick = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        if (uri != null) vm.open(uri)
    }
    Column(
        Modifier.fillMaxSize().padding(24.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text(stringResource(R.string.app_name), style = MaterialTheme.typography.headlineLarge)
        Spacer(Modifier.height(8.dp))
        Text(
            stringResource(R.string.home_tagline),
            style = MaterialTheme.typography.bodyLarge,
            textAlign = TextAlign.Center,
        )
        Spacer(Modifier.height(32.dp))
        Button(onClick = { pick.launch(SPREADSHEET_TYPES) }, Modifier.fillMaxWidth()) {
            Text(stringResource(R.string.open_spreadsheet))
        }
        Spacer(Modifier.height(12.dp))
        OutlinedButton(onClick = vm::newBlank, Modifier.fillMaxWidth()) {
            Text(stringResource(R.string.new_spreadsheet))
        }
        Spacer(Modifier.height(32.dp))
        Text(
            stringResource(R.string.privacy_note),
            style = MaterialTheme.typography.bodySmall,
            textAlign = TextAlign.Center,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(4.dp))
        Text(
            stringResource(R.string.engine_version, vm.engineVersion),
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun SheetScreen(vm: SheetViewModel, state: SheetUiState) {
    val saveAs = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument(SPREADSHEET_TYPES[0]),
    ) { uri -> if (uri != null) vm.saveTo(uri) }

    Column(Modifier.fillMaxSize()) {
        TopAppBar(
            title = { Text(state.fileName, maxLines = 1) },
            navigationIcon = {
                IconButton(onClick = vm::close) {
                    Icon(painterResource(R.drawable.ic_arrow_back), contentDescription = stringResource(R.string.back))
                }
            },
            actions = {
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
