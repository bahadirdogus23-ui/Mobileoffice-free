package org.mobileofficefree.app

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Canvas as AndroidCanvas
import android.graphics.Matrix
import android.graphics.Paint
import android.media.ExifInterface
import android.net.Uri
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import androidx.core.content.FileProvider
import androidx.core.graphics.createBitmap
import java.io.ByteArrayOutputStream
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.mobileofficefree.engine.NamedFile
import org.mobileofficefree.engine.imagesToPdf

/** One photographed page: the photo, a small copy to edit on, where the page is and its look. */
private data class ScanPage(
    val file: File,
    val preview: Bitmap,
    /** Turns of 90° clockwise applied on top of the photo's own orientation. */
    val turns: Int = 0,
    /** Top-left, top-right, bottom-right, bottom-left as fractions of the (turned) photo. */
    val corners: List<Pair<Float, Float>> = ScanImage.defaultCorners,
    val look: ScanLook = ScanLook.DOCUMENT,
)

/**
 * Document scanning: photos from the phone's camera app (or the gallery), each page's corners
 * moved onto the paper, straightened and cleaned up, then saved as one PDF and opened.
 * Returns the action that starts it.
 */
@Composable
fun rememberScanner(onOpen: (Uri) -> Unit): () -> Unit {
    var open by remember { mutableStateOf(false) }
    if (open) ScanDialog(onClose = { open = false }, onOpen = onOpen)
    return { open = true }
}

@Composable
private fun ScanDialog(onClose: () -> Unit, onOpen: (Uri) -> Unit) {
    val context = LocalContext.current
    val resources = LocalResources.current
    val scope = rememberCoroutineScope()
    val dir = remember { File(context.cacheDir, "scan").apply { mkdirs() } }
    val pages = remember { mutableStateListOf<ScanPage>() }
    var current by remember { mutableIntStateOf(0) }
    var working by remember { mutableStateOf(false) }
    var shot by remember { mutableStateOf<File?>(null) }
    val close = {
        dir.listFiles()?.forEach { it.delete() }
        onClose()
    }

    val camera = rememberLauncherForActivityResult(ActivityResultContracts.TakePicture()) { ok ->
        val file = shot
        shot = null
        if (ok && file != null && file.length() > 0) {
            preview(file)?.let {
                pages += ScanPage(file, it)
                current = pages.lastIndex
            }
        } else {
            file?.delete()
        }
    }
    val takePhoto: () -> Unit = {
        val file = File(dir, "sayfa-${System.nanoTime()}.jpg")
        shot = file
        val uri = FileProvider.getUriForFile(context, "${context.packageName}.files", file)
        runCatching { camera.launch(uri) }.onFailure {
            Toast.makeText(context, R.string.scan_no_camera, Toast.LENGTH_LONG).show()
        }
    }
    val gallery = rememberLauncherForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
        for (uri in uris) {
            val file = File(dir, "sayfa-${System.nanoTime()}.jpg")
            runCatching {
                context.contentResolver.openInputStream(uri)?.use { input -> file.outputStream().use { input.copyTo(it) } }
            }
            preview(file)?.let { pages += ScanPage(file, it) } ?: file.delete()
        }
        if (pages.isNotEmpty()) current = pages.lastIndex
    }
    val save = rememberLauncherForActivityResult(CreateFile()) { dest ->
        if (dest == null) return@rememberLauncherForActivityResult
        working = true
        val all = pages.toList()
        scope.launch {
            val result = withContext(Dispatchers.Default) {
                runCatching {
                    val files = all.mapIndexed { i, p -> NamedFile("sayfa-${i + 1}.jpg", finished(p)) }
                    val pdf = imagesToPdf(files, true)
                    context.contentResolver.openOutputStream(dest, "wt")?.use { it.write(pdf) } ?: error("cannot write")
                }
            }
            working = false
            result.fold(
                onSuccess = {
                    close()
                    onOpen(dest)
                },
                onFailure = { Toast.makeText(context, resources.getString(R.string.save_failed, it.message ?: ""), Toast.LENGTH_LONG).show() },
            )
        }
    }

    Dialog(onDismissRequest = close, properties = DialogProperties(usePlatformDefaultWidth = false, decorFitsSystemWindows = false)) {
        Surface(Modifier.fillMaxSize(), color = Color(0xFF202020)) {
            Column(Modifier.fillMaxSize().statusBarsPadding().navigationBarsPadding()) {
                Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp), verticalAlignment = Alignment.CenterVertically) {
                    TextButton(onClick = close) { Text(stringResource(R.string.close), color = Color.White) }
                    Text(
                        if (pages.isEmpty()) stringResource(R.string.scan_title) else stringResource(R.string.page_counter, current + 1, pages.size),
                        color = Color.White,
                        style = MaterialTheme.typography.titleMedium,
                        modifier = Modifier.weight(1f),
                    )
                    Button(
                        onClick = {
                            val stamp = SimpleDateFormat("yyyyMMdd-HHmm", Locale.ROOT).format(Date())
                            save.launch(CreateFile.Request("Tarama-$stamp.pdf", "application/pdf"))
                        },
                        enabled = pages.isNotEmpty() && !working,
                    ) { Text(stringResource(R.string.scan_make_pdf)) }
                }
                if (working) LinearProgressIndicator(Modifier.fillMaxWidth())
                Box(Modifier.weight(1f).fillMaxWidth().padding(12.dp), contentAlignment = Alignment.Center) {
                    val page = pages.getOrNull(current)
                    if (page == null) {
                        Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(12.dp)) {
                            Text(stringResource(R.string.scan_hint), color = Color.White, style = MaterialTheme.typography.bodyLarge)
                            Button(onClick = takePhoto) { Text(stringResource(R.string.scan_take)) }
                            OutlinedButton(onClick = { gallery.launch(arrayOf("image/*")) }) { Text(stringResource(R.string.scan_from_gallery), color = Color.White) }
                        }
                    } else {
                        CornerEditor(page) { corners -> pages[current] = page.copy(corners = corners) }
                    }
                }
                pages.getOrNull(current)?.let { page ->
                    Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 8.dp), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        for (look in ScanLook.entries) {
                            FilterChip(selected = page.look == look, onClick = { pages[current] = page.copy(look = look) }, label = { Text(stringResource(look.label)) })
                        }
                        TextButton(onClick = { pages[current] = turned(page) }) { Text(stringResource(R.string.rotate_right), color = Color.White) }
                        TextButton(onClick = {
                            page.file.delete()
                            pages.removeAt(current)
                            current = current.coerceAtMost(pages.lastIndex).coerceAtLeast(0)
                        }) { Text(stringResource(R.string.delete), color = Color.White) }
                    }
                    Row(
                        Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(8.dp),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        pages.forEachIndexed { i, p ->
                            Image(
                                p.preview.asImageBitmap(),
                                contentDescription = stringResource(R.string.page_counter, i + 1, pages.size),
                                contentScale = ContentScale.Crop,
                                modifier = Modifier
                                    .size(48.dp, 64.dp)
                                    .border(if (i == current) 3.dp else 1.dp, if (i == current) MaterialTheme.colorScheme.primary else Color.Gray)
                                    .clickable { current = i },
                            )
                        }
                        Button(onClick = takePhoto) { Text("+ " + stringResource(R.string.scan_take)) }
                        OutlinedButton(onClick = { gallery.launch(arrayOf("image/*")) }) { Text(stringResource(R.string.scan_from_gallery), color = Color.White) }
                    }
                }
            }
        }
    }
}

/** The page's photo with its corners as handles to drag onto the paper's corners. */
@Composable
private fun CornerEditor(page: ScanPage, onCorners: (List<Pair<Float, Float>>) -> Unit) {
    val bmp = remember(page.preview, page.turns) { rotated(page.preview, page.turns) }
    val reach = with(LocalDensity.current) { 48.dp.toPx() }
    BoxWithConstraints(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        val scale = minOf(constraints.maxWidth / bmp.width.toFloat(), constraints.maxHeight / bmp.height.toFloat())
        val w = bmp.width * scale
        val h = bmp.height * scale
        val density = LocalDensity.current
        Box(Modifier.width(with(density) { w.toDp() }).height(with(density) { h.toDp() })) {
            Image(bmp.asImageBitmap(), contentDescription = null, modifier = Modifier.fillMaxSize())
            var dragging by remember { mutableStateOf<Int?>(null) }
            // The gesture outlives recompositions: it must read the corners as they are now.
            val corners by rememberUpdatedState(page.corners)
            val update by rememberUpdatedState(onCorners)
            Canvas(
                Modifier.fillMaxSize().pointerInput(page.file, page.turns) {
                    detectDragGestures(
                        onDragStart = { at ->
                            dragging = ScanImage.nearestCorner(corners.map { it.first * size.width to it.second * size.height }, at.x, at.y, reach)
                        },
                        onDragEnd = { dragging = null },
                        onDragCancel = { dragging = null },
                    ) { change, _ ->
                        val i = dragging ?: return@detectDragGestures
                        change.consume()
                        val p = change.position
                        update(corners.toMutableList().also { it[i] = (p.x / size.width).coerceIn(0f, 1f) to (p.y / size.height).coerceIn(0f, 1f) })
                    }
                },
            ) {
                val pts = corners.map { Offset(it.first * size.width, it.second * size.height) }
                val path = Path().apply {
                    moveTo(pts[0].x, pts[0].y)
                    for (p in pts.drop(1)) lineTo(p.x, p.y)
                    close()
                }
                drawPath(path, Color(0x332196F3))
                drawPath(path, Color(0xFF2196F3), style = Stroke(width = 3.dp.toPx()))
                for (p in pts) {
                    drawCircle(Color.White, radius = 14.dp.toPx(), center = p)
                    drawCircle(Color(0xFF2196F3), radius = 14.dp.toPx(), center = p, style = Stroke(width = 3.dp.toPx()))
                }
            }
        }
    }
}

/** A page turned 90° clockwise, its corners turned with it. */
private fun turned(page: ScanPage): ScanPage {
    // (x, y) on the old picture lands at (1 - y, x); the corner order moves on by one so the
    // top-left handle is again the one nearest the top left.
    val moved = page.corners.map { (x, y) -> (1 - y) to x }
    return page.copy(turns = (page.turns + 1) % 4, corners = listOf(moved[3], moved[0], moved[1], moved[2]))
}

/** A copy of [bmp] turned [turns] × 90° clockwise. */
private fun rotated(bmp: Bitmap, turns: Int): Bitmap =
    if (turns % 4 == 0) bmp else Bitmap.createBitmap(bmp, 0, 0, bmp.width, bmp.height, Matrix().apply { postRotate(90f * turns) }, true)

/** The photo at [file] upright (its camera orientation applied), at most [maxSide] pixels long. */
private fun load(file: File, maxSide: Int): Bitmap? {
    val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
    BitmapFactory.decodeFile(file.path, bounds)
    if (bounds.outWidth <= 0 || bounds.outHeight <= 0) return null
    var sample = 1
    while (maxOf(bounds.outWidth, bounds.outHeight) / (sample * 2) >= maxSide) sample *= 2
    val bmp = BitmapFactory.decodeFile(file.path, BitmapFactory.Options().apply { inSampleSize = sample }) ?: return null
    val degrees = runCatching {
        when (ExifInterface(file.path).getAttributeInt(ExifInterface.TAG_ORIENTATION, ExifInterface.ORIENTATION_NORMAL)) {
            ExifInterface.ORIENTATION_ROTATE_90 -> 90f
            ExifInterface.ORIENTATION_ROTATE_180 -> 180f
            ExifInterface.ORIENTATION_ROTATE_270 -> 270f
            else -> 0f
        }
    }.getOrDefault(0f)
    val upright = if (degrees == 0f) bmp else Bitmap.createBitmap(bmp, 0, 0, bmp.width, bmp.height, Matrix().apply { postRotate(degrees) }, true)
    val k = maxSide / maxOf(upright.width, upright.height).toFloat()
    return if (k >= 1f) upright else Bitmap.createScaledBitmap(upright, (upright.width * k).toInt().coerceAtLeast(1), (upright.height * k).toInt().coerceAtLeast(1), true)
}

private fun preview(file: File): Bitmap? = load(file, PREVIEW_SIDE)

/** The page straightened, cleaned up and compressed as a JPEG for the PDF. */
private fun finished(page: ScanPage): ByteArray {
    val photo = rotated(load(page.file, PAGE_SIDE) ?: error("the photo could not be read"), page.turns)
    val pts = page.corners.map { (x, y) -> x * photo.width to y * photo.height }
    val (w, h) = ScanImage.outputSize(pts)
    val out = createBitmap(w, h)
    val m = Matrix()
    m.setPolyToPoly(
        floatArrayOf(pts[0].first, pts[0].second, pts[1].first, pts[1].second, pts[2].first, pts[2].second, pts[3].first, pts[3].second),
        0,
        floatArrayOf(0f, 0f, w.toFloat(), 0f, w.toFloat(), h.toFloat(), 0f, h.toFloat()),
        0,
        4,
    )
    AndroidCanvas(out).apply {
        drawColor(android.graphics.Color.WHITE)
        drawBitmap(photo, m, Paint(Paint.FILTER_BITMAP_FLAG or Paint.ANTI_ALIAS_FLAG))
    }
    photo.recycle()
    if (page.look != ScanLook.COLOR) {
        val px = IntArray(w * h)
        out.getPixels(px, 0, w, 0, 0, w, h)
        ScanImage.applyLook(px, w, h, page.look)
        out.setPixels(px, 0, w, 0, 0, w, h)
    }
    val bytes = ByteArrayOutputStream()
    out.compress(Bitmap.CompressFormat.JPEG, if (page.look == ScanLook.COLOR) 85 else 80, bytes)
    out.recycle()
    return bytes.toByteArray()
}

/** Pictures to edit corners on: small enough to keep many pages in memory. */
private const val PREVIEW_SIDE = 1_400
/** Pictures in the PDF: about 250 dpi on A4, sharp text, a few hundred KB a page. */
private const val PAGE_SIDE = 2_600
