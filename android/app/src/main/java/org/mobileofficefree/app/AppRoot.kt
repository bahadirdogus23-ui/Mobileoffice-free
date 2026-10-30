package org.mobileofficefree.app

import android.net.Uri
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
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
import java.util.Locale

/** Routes between the home screen and the open document, and shows one-shot messages. */
@Composable
fun AppRoot(sheet: SheetViewModel, word: WordViewModel, pdf: PdfViewModel, slides: SlidesViewModel, cad: CadViewModel, image: ImageViewModel, text: TextViewModel, archive: ArchiveViewModel, invoice: InvoiceViewModel, openRequest: Uri?, onOpenHandled: () -> Unit) {
    val context = LocalContext.current
    val resources = LocalResources.current
    val snackbar = remember { SnackbarHostState() }
    val reading = remember { ReadingSettings(context) }
    var notice by remember { mutableStateOf<String?>(null) }
    var cadNotesSeen by remember(cad.state?.uri) { mutableStateOf(false) }
    var showDiagnostics by remember { mutableStateOf(false) }
    // A crash report from the last run, offered once for sharing.
    var crashReport by remember { mutableStateOf(Diagnostics.takeCrashReport(context)) }

    val openAny: (Uri) -> Unit = { uri ->
        val info = runCatching { FileInfo.of(context.contentResolver, uri) }.getOrNull()
        when (info?.kind) {
            DocumentKind.WORD -> word.open(uri)
            DocumentKind.SHEET -> sheet.open(uri)
            DocumentKind.PDF -> pdf.open(uri)
            DocumentKind.CAD -> cad.open(uri)
            DocumentKind.IMAGE -> image.open(uri)
            DocumentKind.TEXT -> text.open(uri)
            DocumentKind.ARCHIVE -> archive.open(uri)
            DocumentKind.INVOICE -> invoice.open(uri)
            // DeckCraft reads PPTX/PPSX/POTX; legacy .ppt and .odp come with phase 4 (plan §6).
            DocumentKind.SLIDES -> if (info.typedName.substringAfterLast('.').lowercase(Locale.ROOT) in setOf("pptx", "ppsx", "potx")) {
                slides.open(uri)
            } else {
                notice = resources.getString(R.string.coming_soon, info.name)
            }
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
    for (paged in listOf(word, pdf, slides)) {
        LaunchedEffect(paged, paged.error) {
            paged.error?.let {
                snackbar.showSnackbar(resources.getString(R.string.open_failed, it))
                paged.consumeError()
            }
        }
    }
    LaunchedEffect(cad.error) {
        cad.error?.let {
            snackbar.showSnackbar(resources.getString(R.string.open_failed, it))
            cad.consumeError()
        }
    }
    LaunchedEffect(image.error) {
        image.error?.let {
            snackbar.showSnackbar(resources.getString(R.string.open_failed, it))
            image.consumeError()
        }
    }
    LaunchedEffect(invoice.error) {
        invoice.error?.let {
            snackbar.showSnackbar(resources.getString(R.string.open_failed, it))
            invoice.consumeError()
        }
    }
    LaunchedEffect(archive.error) {
        archive.error?.let {
            snackbar.showSnackbar(resources.getString(R.string.open_failed, it))
            archive.consumeError()
        }
    }
    LaunchedEffect(text.error) {
        text.error?.let {
            snackbar.showSnackbar(resources.getString(R.string.open_failed, it))
            text.consumeError()
        }
    }
    LaunchedEffect(notice) {
        notice?.let {
            snackbar.showSnackbar(it)
            notice = null
        }
    }

    // No insets here: each screen's top bar keeps clear of the status bar and its bottom bar (or
    // padding) of the navigation bar. Adding them here as well left empty bands at the top and bottom.
    Scaffold(contentWindowInsets = WindowInsets(0, 0, 0, 0), snackbarHost = { SnackbarHost(snackbar) }) { padding ->
        Box(Modifier.padding(padding).fillMaxSize()) {
            val sheetScreen = sheet.screen
            val wordState = word.state
            val pdfState = pdf.state
            val slidesState = slides.state
            val cadState = cad.state
            val imageState = image.state
            val textState = text.state
            val archiveState = archive.state
            val invoiceState = invoice.state
            when {
                word.loading || pdf.loading || slides.loading || cad.loading || image.loading || text.loading || archive.loading || invoice.loading || sheetScreen == Screen.Loading -> Loading()
                wordState != null -> PagedScreen(word, wordState, reading)
                pdfState != null -> PagedScreen(pdf, pdfState, reading)
                slidesState != null -> PagedScreen(slides, slidesState, reading)
                cadState != null -> CadScreen(cad, cadState)
                imageState != null -> ImageScreen(image, imageState)
                textState != null -> TextScreen(text, textState, reading)
                invoiceState != null -> InvoiceScreen(invoice, invoiceState)
                sheetScreen is Screen.Sheet -> SheetScreen(sheet, sheetScreen.state)
                // Below the viewers: closing a file opened from the archive comes back to its list.
                archiveState != null -> ArchiveScreen(archive, archiveState, onOpen = openAny)
                showDiagnostics -> DiagnosticsScreen(onBack = { showDiagnostics = false })
                else -> HomeScreen(
                    onOpen = openAny,
                    onNewSheet = sheet::newBlank,
                    onDiagnostics = { showDiagnostics = true },
                    engineVersion = sheet.engineVersion,
                )
            }
        }
    }

    cad.state?.notes?.takeIf { it.isNotEmpty() && !cadNotesSeen }?.let { notes ->
        AlertDialog(
            onDismissRequest = { cadNotesSeen = true },
            confirmButton = { TextButton(onClick = { cadNotesSeen = true }) { Text(stringResource(R.string.ok)) } },
            title = { Text(stringResource(R.string.import_notes)) },
            text = { Text(notes.take(5).joinToString("\n\n")) },
        )
    }
    crashReport?.let { report ->
        AlertDialog(
            onDismissRequest = { crashReport = null },
            title = { Text(stringResource(R.string.crash_title)) },
            text = { Text(stringResource(R.string.crash_body)) },
            confirmButton = {
                TextButton(onClick = {
                    crashReport = null
                    shareText(context, report)
                }) { Text(stringResource(R.string.share)) }
            },
            dismissButton = { TextButton(onClick = { crashReport = null }) { Text(stringResource(R.string.close)) } },
        )
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
