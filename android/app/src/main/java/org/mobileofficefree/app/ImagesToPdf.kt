package org.mobileofficefree.app

import android.net.Uri
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.mobileofficefree.engine.NamedFile
import org.mobileofficefree.engine.imagesToPdf

/**
 * "Pictures to PDF": pick pictures, choose A4 pages or the pictures' own size, save the new PDF
 * and open it. Returns the action that starts it.
 */
@Composable
fun rememberImagesToPdf(onOpen: (Uri) -> Unit): () -> Unit {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var picked by remember { mutableStateOf<List<Uri>>(emptyList()) }
    var fitA4 by remember { mutableStateOf(true) }
    var working by remember { mutableStateOf(false) }
    val save = rememberLauncherForActivityResult(CreateFile()) { dest ->
        val images = picked
        picked = emptyList()
        if (dest == null || images.isEmpty()) return@rememberLauncherForActivityResult
        working = true
        scope.launch {
            val result = withContext(Dispatchers.IO) {
                runCatching {
                    val resolver = context.contentResolver
                    val files = images.map { uri ->
                        val name = runCatching { FileInfo.of(resolver, uri).name }.getOrDefault("resim")
                        NamedFile(name, resolver.openInputStream(uri)?.use { it.readBytes() } ?: error("no data"))
                    }
                    val pdf = imagesToPdf(files, fitA4)
                    resolver.openOutputStream(dest, "wt")?.use { it.write(pdf) } ?: error("cannot write")
                }
            }
            working = false
            result.fold(
                onSuccess = { onOpen(dest) },
                onFailure = { Toast.makeText(context, context.getString(R.string.save_failed, it.message ?: ""), Toast.LENGTH_LONG).show() },
            )
        }
    }
    var choosing by remember { mutableStateOf(false) }
    val pick = rememberLauncherForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
        if (uris.isNotEmpty()) {
            picked = uris
            choosing = true
        }
    }
    if (choosing) {
        AlertDialog(
            onDismissRequest = {
                choosing = false
                picked = emptyList()
            },
            title = { Text(stringResource(R.string.images_to_pdf)) },
            text = {
                Column {
                    Text(pluralStringResource(R.plurals.images_picked, picked.size, picked.size))
                    for ((a4, label) in listOf(true to R.string.images_a4, false to R.string.images_own_size)) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            modifier = Modifier.fillMaxWidth().clickable { fitA4 = a4 },
                        ) {
                            RadioButton(selected = fitA4 == a4, onClick = { fitA4 = a4 })
                            Text(stringResource(label))
                        }
                    }
                }
            },
            confirmButton = {
                TextButton(onClick = {
                    choosing = false
                    save.launch(CreateFile.Request(context.getString(R.string.images_pdf_name), "application/pdf"))
                }) { Text(stringResource(R.string.create)) }
            },
            dismissButton = {
                TextButton(onClick = {
                    choosing = false
                    picked = emptyList()
                }) { Text(stringResource(R.string.cancel)) }
            },
        )
    }
    if (working) {
        AlertDialog(
            onDismissRequest = {},
            confirmButton = {},
            text = { Text(stringResource(R.string.images_working)) },
        )
    }
    return { if (!working) pick.launch(arrayOf("image/*")) }
}
