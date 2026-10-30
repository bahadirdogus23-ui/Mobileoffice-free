package org.mobileofficefree.app

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.Checkbox
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties

/** A hand-drawn signature: strokes of points in any one unit (the pad's pixels). */
typealias Signature = List<List<Offset>>

/**
 * A white pad to draw a signature on with a finger, above the signatures kept from before;
 * [onDone] gets the strokes (a new one is kept when the person leaves "keep" ticked).
 */
@Composable
fun SignaturePad(store: SavedSignatures, onDismiss: () -> Unit, onDone: (Signature) -> Unit) {
    val strokes = remember { mutableStateListOf<List<Offset>>() }
    val drawing = remember { mutableStateListOf<Offset>() }
    val saved = remember { mutableStateListOf<Signature>().apply { addAll(store.load()) } }
    var keep by remember { mutableStateOf(true) }
    Dialog(onDismissRequest = onDismiss, properties = DialogProperties(usePlatformDefaultWidth = false)) {
        Surface(shape = MaterialTheme.shapes.large, modifier = Modifier.fillMaxWidth().padding(16.dp)) {
            Column(Modifier.padding(16.dp)) {
                if (saved.isNotEmpty()) {
                    Text(stringResource(R.string.signature_saved), style = MaterialTheme.typography.titleMedium)
                    Row(Modifier.horizontalScroll(rememberScrollState()).padding(vertical = 8.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        saved.forEachIndexed { i, sig ->
                            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                                SignaturePreview(sig, Modifier.width(120.dp).height(56.dp).clickable { onDone(sig) })
                                TextButton(onClick = {
                                    store.remove(i)
                                    saved.removeAt(i)
                                }) { Text(stringResource(R.string.delete)) }
                            }
                        }
                    }
                }
                Text(stringResource(R.string.signature_draw), style = MaterialTheme.typography.titleLarge)
                Canvas(
                    Modifier
                        .padding(vertical = 12.dp)
                        .fillMaxWidth()
                        .aspectRatio(PAD_ASPECT)
                        .background(Color.White)
                        .border(1.dp, Color(0x33000000))
                        .pointerInput(Unit) {
                            awaitEachGesture {
                                val down = awaitFirstDown()
                                drawing.clear()
                                drawing.add(down.position)
                                do {
                                    val event = awaitPointerEvent()
                                    event.changes.firstOrNull { it.id == down.id }?.let { change ->
                                        if (change.positionChange() != Offset.Zero) drawing.add(change.position)
                                        change.consume()
                                    }
                                } while (event.changes.any { it.id == down.id && it.pressed })
                                strokes.add(drawing.toList())
                                drawing.clear()
                            }
                        },
                ) {
                    // A line to sign on.
                    val base = size.height * 0.75f
                    drawLine(Color(0x55000000), Offset(size.width * 0.06f, base), Offset(size.width * 0.94f, base), 2f)
                    val ink = Stroke(width = 3.dp.toPx(), cap = StrokeCap.Round, join = StrokeJoin.Round)
                    for (s in strokes + listOf(drawing.toList())) {
                        if (s.isEmpty()) continue
                        val path = Path().apply {
                            moveTo(s[0].x, s[0].y)
                            if (s.size == 1) lineTo(s[0].x + 0.1f, s[0].y)
                            for (p in s.drop(1)) lineTo(p.x, p.y)
                        }
                        drawPath(path, Color.Black, style = ink)
                    }
                }
                Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.clickable { keep = !keep }) {
                    Checkbox(checked = keep, onCheckedChange = { keep = it })
                    Text(stringResource(R.string.signature_keep))
                }
                Row(horizontalArrangement = Arrangement.End, modifier = Modifier.fillMaxWidth()) {
                    TextButton(onClick = { strokes.clear() }, enabled = strokes.isNotEmpty()) { Text(stringResource(R.string.clear)) }
                    TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel)) }
                    TextButton(onClick = {
                        val drawn = strokes.toList()
                        if (keep) store.add(drawn)
                        onDone(drawn)
                    }, enabled = strokes.isNotEmpty()) { Text(stringResource(R.string.done)) }
                }
            }
        }
    }
}

/** A kept signature drawn small, black on white. */
@Composable
private fun SignaturePreview(signature: Signature, modifier: Modifier) {
    Canvas(modifier.background(Color.White).border(1.dp, Color(0x33000000)).padding(6.dp)) {
        val points = signature.flatten()
        if (points.isEmpty()) return@Canvas
        val left = points.minOf { it.x }
        val top = points.minOf { it.y }
        val w = (points.maxOf { it.x } - left).coerceAtLeast(1e-3f)
        val h = (points.maxOf { it.y } - top).coerceAtLeast(1e-3f)
        val k = minOf(size.width / w, size.height / h)
        val dx = (size.width - w * k) / 2
        val dy = (size.height - h * k) / 2
        val ink = Stroke(width = 1.5.dp.toPx(), cap = StrokeCap.Round, join = StrokeJoin.Round)
        for (s in signature) {
            if (s.isEmpty()) continue
            val path = Path().apply {
                moveTo((s[0].x - left) * k + dx, (s[0].y - top) * k + dy)
                for (p in s.drop(1)) lineTo((p.x - left) * k + dx, (p.y - top) * k + dy)
            }
            drawPath(path, Color.Black, style = ink)
        }
    }
}

/**
 * [signature] fitted into a box a third of the page wide, centred on [at] (page points, y down),
 * as flat `x, y` fractions of the page, the way the engine takes them.
 */
fun placeSignature(signature: Signature, at: Offset, pageWidth: Float, pageHeight: Float): List<List<Double>> {
    val points = signature.flatten()
    if (points.isEmpty()) return emptyList()
    val left = points.minOf { it.x }
    val top = points.minOf { it.y }
    val right = points.maxOf { it.x }
    val bottom = points.maxOf { it.y }
    val boxWidth = pageWidth / 3f
    val k = minOf(boxWidth / (right - left).coerceAtLeast(1f), boxWidth / PAD_ASPECT / (bottom - top).coerceAtLeast(1f))
    val cx = (left + right) / 2f
    val cy = (top + bottom) / 2f
    return signature.filter { it.isNotEmpty() }.map { stroke ->
        stroke.flatMap { p ->
            val x = (at.x + (p.x - cx) * k) / pageWidth
            val y = (at.y + (p.y - cy) * k) / pageHeight
            listOf(x.coerceIn(0f, 1f).toDouble(), y.coerceIn(0f, 1f).toDouble())
        }
    }
}

private const val PAD_ASPECT = 2.5f

/**
 * Signatures the person chose to keep, in the app's private files (never shared, gone with the
 * app's data). One per line: strokes split by `;`, points `x,y` split by spaces, scaled to 0–1.
 */
class SavedSignatures(private val file: java.io.File) {
    fun load(): List<Signature> = runCatching { decode(file.readText()) }.getOrDefault(emptyList())

    /** Keeps [signature] first, at most [MAX] in all. */
    fun add(signature: Signature) = write(listOf(normalized(signature)) + load().take(MAX - 1))

    fun remove(index: Int) = write(load().filterIndexed { i, _ -> i != index })

    private fun write(all: List<Signature>) {
        runCatching { file.writeText(encode(all)) }
    }

    companion object {
        const val MAX = 3

        fun encode(all: List<Signature>): String = all.joinToString("\n") { sig ->
            sig.filter { it.isNotEmpty() }.joinToString(";") { stroke -> stroke.joinToString(" ") { "${it.x},${it.y}" } }
        }

        fun decode(text: String): List<Signature> = text.lines().filter { it.isNotBlank() }.mapNotNull { line ->
            runCatching {
                line.split(';').map { stroke ->
                    stroke.trim().split(' ').map { p ->
                        val (x, y) = p.split(',')
                        Offset(x.toFloat(), y.toFloat())
                    }
                }
            }.getOrNull()
        }

        /** The drawing moved to the top left and scaled so its longer side is 1. */
        fun normalized(signature: Signature): Signature {
            val points = signature.flatten()
            if (points.isEmpty()) return signature
            val left = points.minOf { it.x }
            val top = points.minOf { it.y }
            val side = maxOf(points.maxOf { it.x } - left, points.maxOf { it.y } - top).coerceAtLeast(1e-3f)
            return signature.filter { it.isNotEmpty() }.map { s -> s.map { Offset((it.x - left) / side, (it.y - top) / side) } }
        }
    }
}
