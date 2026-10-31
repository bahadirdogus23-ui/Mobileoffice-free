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
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import androidx.compose.foundation.Image
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.produceState
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectDragGesturesAfterLongPress
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.zIndex
import kotlin.math.roundToInt
import org.mobileofficefree.engine.NamedFile
import org.mobileofficefree.engine.imagesToPdf

/**
 * "Pictures to PDF": pick pictures, choose A4 pages or the pictures' own size, save the new PDF
 * and open it. Returns the action that starts it.
 */
@Composable
fun rememberImagesToPdf(onOpen: (Uri) -> Unit): () -> Unit {
    val context = LocalContext.current
    val resources = LocalResources.current
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
                onFailure = { Toast.makeText(context, resources.getString(R.string.save_failed, it.message ?: ""), Toast.LENGTH_LONG).show() },
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
                    // The pages in order: hold one and drag it to move it; ✕ takes it out.
                    Text(stringResource(R.string.pages_drag_hint), style = MaterialTheme.typography.bodySmall)
                    PageOrder(picked, onChange = { picked = it })
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
                TextButton(enabled = picked.isNotEmpty(), onClick = {
                    choosing = false
                    save.launch(CreateFile.Request(resources.getString(R.string.images_pdf_name), "application/pdf"))
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

/** [list] with the item at [from] moved to [to]. */
internal fun <T> moved(list: List<T>, from: Int, to: Int): List<T> {
    if (from !in list.indices) return list
    val out = list.toMutableList()
    val item = out.removeAt(from)
    out.add(to.coerceIn(0, out.size), item)
    return out
}

/** The picked pictures as a row of pages; a long press picks one up to drag it elsewhere. */
@Composable
private fun PageOrder(pages: List<Uri>, onChange: (List<Uri>) -> Unit) {
    val step = with(LocalDensity.current) { (THUMB_WIDTH + THUMB_GAP).toPx() }
    var dragging by remember { mutableStateOf<Uri?>(null) }
    var offset by remember { mutableFloatStateOf(0f) }
    val current by rememberUpdatedState(pages)
    Row(
        Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(vertical = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(THUMB_GAP),
    ) {
        current.forEachIndexed { i, uri ->
            val lifted = dragging == uri
            Box(
                Modifier
                    .zIndex(if (lifted) 1f else 0f)
                    .graphicsLayer {
                        translationX = if (lifted) offset else 0f
                        scaleX = if (lifted) 1.06f else 1f
                        scaleY = if (lifted) 1.06f else 1f
                        shadowElevation = if (lifted) 12f else 0f
                    }
                    .pointerInput(uri) {
                        detectDragGesturesAfterLongPress(
                            onDragStart = { dragging = uri; offset = 0f },
                            onDragEnd = {
                                val from = current.indexOf(uri)
                                val to = from + (offset / step).roundToInt()
                                dragging = null
                                offset = 0f
                                if (from >= 0 && to != from) onChange(moved(current, from, to))
                            },
                            onDragCancel = { dragging = null; offset = 0f },
                        ) { change, amount ->
                            change.consume()
                            offset += amount.x
                        }
                    },
            ) {
                PageThumb(uri, i + 1, onRemove = { onChange(current.filterIndexed { k, _ -> k != i }) })
            }
        }
    }
}

/** One picked picture as page [number], with a button to take it out. */
@Composable
private fun PageThumb(uri: Uri, number: Int, onRemove: () -> Unit) {
    val context = LocalContext.current
    val thumb by produceState<ImageBitmap?>(null, uri) {
        value = withContext(Dispatchers.IO) { runCatching { thumbnail(context, uri) }.getOrNull()?.asImageBitmap() }
    }
    Column(horizontalAlignment = Alignment.CenterHorizontally, modifier = Modifier.width(THUMB_WIDTH)) {
        Box(
            Modifier.size(THUMB_WIDTH, 112.dp).clip(RoundedCornerShape(6.dp)).background(MaterialTheme.colorScheme.surfaceVariant)
                .border(1.dp, MaterialTheme.colorScheme.outlineVariant, RoundedCornerShape(6.dp)),
            contentAlignment = Alignment.Center,
        ) {
            thumb?.let { Image(it, contentDescription = null, contentScale = ContentScale.Fit, modifier = Modifier.fillMaxSize()) }
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(number.toString(), style = MaterialTheme.typography.labelLarge)
            TextButton(onClick = onRemove, contentPadding = PaddingValues(4.dp)) { Text("✕") }
        }
    }
}

private val THUMB_WIDTH = 84.dp
private val THUMB_GAP = 10.dp

/** A small copy of the picture at [uri] for the page list. */
private fun thumbnail(context: android.content.Context, uri: Uri): Bitmap? {
    val resolver = context.contentResolver
    val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
    resolver.openInputStream(uri)?.use { BitmapFactory.decodeStream(it, null, bounds) }
    var sample = 1
    while (maxOf(bounds.outWidth, bounds.outHeight) / (sample * 2) >= THUMB_SIDE) sample *= 2
    return resolver.openInputStream(uri)?.use { BitmapFactory.decodeStream(it, null, BitmapFactory.Options().apply { inSampleSize = sample }) }
}

private const val THUMB_SIDE = 240
