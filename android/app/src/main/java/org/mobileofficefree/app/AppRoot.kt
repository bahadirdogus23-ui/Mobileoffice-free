package org.mobileofficefree.app

import android.net.Uri
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp

/** Routes between the home screen and the open document, and shows one-shot messages. */
@Composable
fun AppRoot(sheet: SheetViewModel, word: WordViewModel, pdf: PdfViewModel, openRequest: Uri?, onOpenHandled: () -> Unit) {
    val context = LocalContext.current
    val resources = LocalResources.current
    val snackbar = remember { SnackbarHostState() }
    var notice by remember { mutableStateOf<String?>(null) }

    val openAny: (Uri) -> Unit = { uri ->
        val info = runCatching { FileInfo.of(context.contentResolver, uri) }.getOrNull()
        when (info?.kind) {
            DocumentKind.WORD -> word.open(uri)
            DocumentKind.SHEET -> sheet.open(uri)
            DocumentKind.PDF -> pdf.open(uri)
            DocumentKind.SLIDES -> notice = resources.getString(R.string.coming_soon, info.name)
            else -> notice = resources.getString(R.string.unsupported_file)
        }
    }

    LaunchedEffect(openRequest) {
        openRequest?.let {
            openAny(it)
            onOpenHandled()
        }
    }
    LaunchedEffect(sheet.message) {
        sheet.message?.let {
            snackbar.showSnackbar(it)
            sheet.consumeMessage()
        }
    }
    for (paged in listOf(word, pdf)) {
        LaunchedEffect(paged, paged.error) {
            paged.error?.let {
                snackbar.showSnackbar(resources.getString(R.string.open_failed, it))
                paged.consumeError()
            }
        }
    }
    LaunchedEffect(notice) {
        notice?.let {
            snackbar.showSnackbar(it)
            notice = null
        }
    }

    Scaffold(snackbarHost = { SnackbarHost(snackbar) }) { padding ->
        Box(Modifier.padding(padding).fillMaxSize()) {
            val sheetScreen = sheet.screen
            val wordState = word.state
            val pdfState = pdf.state
            when {
                word.loading || pdf.loading || sheetScreen == Screen.Loading -> Loading()
                wordState != null -> PagedScreen(word, wordState)
                pdfState != null -> PagedScreen(pdf, pdfState)
                sheetScreen is Screen.Sheet -> SheetScreen(sheet, sheetScreen.state)
                else -> HomeScreen(onOpen = openAny, onNewSheet = sheet::newBlank, engineVersion = sheet.engineVersion)
            }
        }
    }

    if (sheet.importNotes.isNotEmpty()) {
        AlertDialog(
            onDismissRequest = sheet::dismissImportNotes,
            confirmButton = { TextButton(onClick = sheet::dismissImportNotes) { Text(stringResource(R.string.ok)) } },
            title = { Text(stringResource(R.string.import_notes)) },
            text = { Text(sheet.importNotes.take(5).joinToString("\n\n")) },
        )
    }
}

@Composable
private fun Loading() {
    Column(Modifier.fillMaxSize(), verticalArrangement = Arrangement.Center, horizontalAlignment = Alignment.CenterHorizontally) {
        CircularProgressIndicator()
        Spacer(Modifier.height(12.dp))
        Text(stringResource(R.string.loading))
    }
}
