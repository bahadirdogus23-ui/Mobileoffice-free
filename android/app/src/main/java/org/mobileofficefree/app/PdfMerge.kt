package org.mobileofficefree.app

import android.net.Uri
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
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
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.mobileofficefree.engine.PdfDocument

/**
 * "Combine PDFs": pick several PDFs, put them in order, save them as one new PDF and open it.
 * The picked files are only read. Returns the action that starts it.
 */
@Composable
fun rememberPdfMerge(onOpen: (Uri) -> Unit): PdfMergeAction {
    val context = LocalContext.current
    val resources = LocalResources.current
    val scope = rememberCoroutineScope()
    var picked by remember { mutableStateOf<List<Pair<Uri, String>>>(emptyList()) }
    var choosing by remember { mutableStateOf(false) }
    var working by remember { mutableStateOf(false) }
    val save = rememberLauncherForActivityResult(CreateFile()) { dest ->
        val files = picked
        picked = emptyList()
        if (dest == null || files.size < 2) return@rememberLauncherForActivityResult
        working = true
        scope.launch {
            val result = withContext(Dispatchers.IO) {
                runCatching {
                    val resolver = context.contentResolver
                    fun bytesOf(uri: Uri) = resolver.openInputStream(uri)?.use { it.readBytes() } ?: error("no data")
                    val (firstUri, firstName) = files.first()
                    val merged = runCatching { PdfDocument.open(bytesOf(firstUri), null) }
                        .getOrElse { throw IllegalStateException(resources.getString(R.string.merge_cannot_read, firstName), it) }
                    try {
                        for ((uri, name) in files.drop(1)) {
                            runCatching { merged.append(bytesOf(uri), null) }
                                .onFailure { throw IllegalStateException(resources.getString(R.string.merge_cannot_read, name), it) }
                        }
                        val out = merged.save()
                        resolver.openOutputStream(dest, "wt")?.use { it.write(out) } ?: error("cannot write")
                    } finally {
                        merged.close()
                    }
                }
            }
            working = false
            result.fold(
                onSuccess = { onOpen(dest) },
                onFailure = { Toast.makeText(context, it.message ?: resources.getString(R.string.save_failed, ""), Toast.LENGTH_LONG).show() },
            )
        }
    }
    val pick = rememberLauncherForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
        if (uris.isNotEmpty()) {
            val resolver = context.contentResolver
            // Picked again from the list: the new files join the end, the same file not twice.
            val added = uris.map { it to runCatching { FileInfo.of(resolver, it).name }.getOrDefault("PDF") }
            picked = (picked + added).distinctBy { it.first }
            choosing = true
        }
    }
    if (choosing) {
        AlertDialog(
            onDismissRequest = {
                choosing = false
                picked = emptyList()
            },
            title = { Text(stringResource(R.string.merge_pdfs)) },
            text = {
                Column {
                    Text(stringResource(R.string.merge_hint), style = MaterialTheme.typography.bodySmall)
                    LazyColumn(Modifier.heightIn(max = 360.dp)) {
                        itemsIndexed(picked, key = { _, p -> p.first.toString() }) { i, (_, name) ->
                            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                                Text("${i + 1}. $name", maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
                                TextButton(onClick = { picked = moved(picked, i, i - 1) }, enabled = i > 0, modifier = Modifier.described(stringResource(R.string.a11y_move_up))) { Text("▲") }
                                TextButton(onClick = { picked = moved(picked, i, i + 1) }, enabled = i < picked.lastIndex, modifier = Modifier.described(stringResource(R.string.a11y_move_down))) { Text("▼") }
                                TextButton(onClick = { picked = picked.filterIndexed { j, _ -> j != i } }, modifier = Modifier.described(stringResource(R.string.a11y_remove))) { Text("✕") }
                            }
                        }
                    }
                    TextButton(onClick = { pick.launch(arrayOf("application/pdf")) }) { Text(stringResource(R.string.merge_add)) }
                    if (picked.size < 2) Text(stringResource(R.string.merge_need_two), color = MaterialTheme.colorScheme.error)
                }
            },
            confirmButton = {
                TextButton(enabled = picked.size >= 2, onClick = {
                    choosing = false
                    save.launch(CreateFile.Request(resources.getString(R.string.merge_file_name), "application/pdf"))
                }) { Text(stringResource(R.string.merge_do)) }
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
        AlertDialog(onDismissRequest = {}, confirmButton = {}, text = { Text(stringResource(R.string.merge_working)) })
    }
    return remember {
        PdfMergeAction(
            pick = { if (!working) pick.launch(arrayOf("application/pdf")) },
            with = { files ->
                if (!working) {
                    picked = files.distinctBy { it.first }
                    choosing = true
                }
            },
        )
    }
}

/** Starts "Combine PDFs": by picking files ([pick]), or with files already chosen ([with], in order). */
class PdfMergeAction(val pick: () -> Unit, val with: (List<Pair<Uri, String>>) -> Unit)
