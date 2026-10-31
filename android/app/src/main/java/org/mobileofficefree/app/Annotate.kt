package org.mobileofficefree.app

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import org.mobileofficefree.engine.Pen
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Slider
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.ui.draw.clip
import kotlin.math.roundToInt
import org.mobileofficefree.engine.PdfTextFont
import org.mobileofficefree.engine.TextLook

/** What a touch on a page does while annotating. */
enum class AnnotTool(val label: Int, val pen: Pen?) {
    BLACK(R.string.pen_black, Pen(0.0, 0.0, 0.0, 1.5, 1.0)),
    RED(R.string.pen_red, Pen(0.85, 0.1, 0.1, 1.5, 1.0)),
    BLUE(R.string.pen_blue, Pen(0.1, 0.25, 0.85, 1.5, 1.0)),
    HIGHLIGHT(R.string.highlighter, Pen(1.0, 0.9, 0.0, 12.0, 0.4)),
    TEXT(R.string.pdf_text, null),
}

/** A stroke not yet written into the PDF: points as fractions of the displayed page. */
class PenStroke(val page: Int, val points: List<Offset>, val pen: Pen) {
    /** Flat `x, y` pairs, the way the engine takes them. */
    fun flat(): List<Double> = points.flatMap { listOf(it.x.toDouble(), it.y.toDouble()) }
}

/** The annotation tools, shown above the pages while annotating. */
@Composable
fun AnnotationBar(tool: AnnotTool, onTool: (AnnotTool) -> Unit, canUndo: Boolean, onUndo: () -> Unit, onDone: () -> Unit) {
    Surface(color = MaterialTheme.colorScheme.surfaceContainer, modifier = Modifier.fillMaxWidth()) {
        Column {
            if (tool == AnnotTool.TEXT) {
                Text(
                    stringResource(R.string.pdf_text_hint),
                    style = MaterialTheme.typography.bodySmall,
                    modifier = Modifier.padding(start = 12.dp, top = 6.dp),
                )
            }
            Row(Modifier.horizontalScroll(rememberScrollState()).padding(horizontal = 8.dp), verticalAlignment = Alignment.CenterVertically) {
                for (t in AnnotTool.entries) {
                    FilterChip(
                        selected = t == tool,
                        onClick = { onTool(t) },
                        label = { Text(stringResource(t.label)) },
                        modifier = Modifier.padding(end = 6.dp),
                    )
                }
                TextButton(onClick = onUndo, enabled = canUndo) { Text(stringResource(R.string.undo)) }
                TextButton(onClick = onDone) { Text(stringResource(R.string.done)) }
            }
        }
    }
}

/**
 * One finger draws on the page (two are left to zooming): each finished stroke goes to [onStroke]
 * as fractions of the page; the stroke being drawn is shown through [live].
 */
fun Modifier.drawStrokes(key: Any, live: (List<Offset>) -> Unit, onStroke: (List<Offset>) -> Unit): Modifier = pointerInput(key) {
    awaitEachGesture {
        val down = awaitFirstDown()
        val w = size.width.coerceAtLeast(1).toFloat()
        val h = size.height.coerceAtLeast(1).toFloat()
        val points = mutableListOf(Offset(down.position.x / w, down.position.y / h))
        down.consume()
        live(points.toList())
        var cancelled = false
        do {
            val event = awaitPointerEvent()
            if (event.changes.count { it.pressed } > 1) cancelled = true
            event.changes.firstOrNull { it.id == down.id }?.let { c ->
                if (!cancelled && c.positionChange() != Offset.Zero) {
                    points += Offset(c.position.x / w, c.position.y / h)
                    live(points.toList())
                }
                if (!cancelled) c.consume()
            }
        } while (event.changes.any { it.id == down.id && it.pressed })
        live(emptyList())
        if (!cancelled) onStroke(points)
    }
}

/** Strokes not yet written, drawn over a page [pageWidth] points wide. */
@Composable
fun PendingStrokes(strokes: List<Pair<List<Offset>, Pen>>, pageWidth: Float) {
    Canvas(Modifier.fillMaxSize()) {
        val k = size.width / pageWidth
        for ((points, pen) in strokes) {
            if (points.isEmpty()) continue
            val path = Path().apply {
                moveTo(points[0].x * size.width, points[0].y * size.height)
                if (points.size == 1) lineTo(points[0].x * size.width + 0.1f, points[0].y * size.height)
                for (p in points.drop(1)) lineTo(p.x * size.width, p.y * size.height)
            }
            drawPath(
                path,
                Color(pen.red.toFloat(), pen.green.toFloat(), pen.blue.toFloat(), pen.opacity.toFloat()),
                style = Stroke(width = (pen.width * k).toFloat().coerceAtLeast(1.5f), cap = StrokeCap.Round, join = StrokeJoin.Round),
            )
        }
    }
}

/** The last look chosen for typed text, kept while the app runs. */
object TextStyleMemory {
    var size by mutableFloatStateOf(14f)
    var font by mutableStateOf(PdfTextFont.SANS)
    var color by mutableIntStateOf(0)
}

/** Colours offered for typed text: black, dark blue, red, green, grey. */
internal val TEXT_STAMP_COLORS = listOf(
    Color(0xFF000000), Color(0xFF1A3A8F), Color(0xFFC62828), Color(0xFF2E7D32), Color(0xFF616161),
)

private val TEXT_FONTS = listOf(
    PdfTextFont.SANS to R.string.font_sans,
    PdfTextFont.SANS_BOLD to R.string.font_sans_bold,
    PdfTextFont.SERIF to R.string.font_serif,
    PdfTextFont.SERIF_BOLD to R.string.font_serif_bold,
    PdfTextFont.MONO to R.string.font_mono,
)

/** Asks for text to type on a page, with its size, typeface and colour. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun TextStampDialog(onDismiss: () -> Unit, onAdd: (String, TextLook) -> Unit) {
    var text by remember { mutableStateOf("") }
    val memory = TextStyleMemory
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.pdf_text_title)) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(value = text, onValueChange = { text = it }, minLines = 2, modifier = Modifier.fillMaxWidth())
                Text(stringResource(R.string.text_point_size, memory.size.roundToInt()), style = MaterialTheme.typography.labelLarge)
                Slider(value = memory.size, onValueChange = { memory.size = it }, valueRange = 6f..48f, steps = 20)
                FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    for ((font, label) in TEXT_FONTS) {
                        FilterChip(selected = memory.font == font, onClick = { memory.font = font }, label = { Text(stringResource(label)) })
                    }
                }
                Row(horizontalArrangement = Arrangement.spacedBy(10.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text(stringResource(R.string.text_color), style = MaterialTheme.typography.labelLarge)
                    TEXT_STAMP_COLORS.forEachIndexed { i, c ->
                        Box(
                            Modifier
                                .size(if (memory.color == i) 34.dp else 28.dp)
                                .clip(CircleShape)
                                .background(c)
                                .border(if (memory.color == i) 3.dp else 1.dp, MaterialTheme.colorScheme.primary, CircleShape)
                                .clickable { memory.color = i },
                        )
                    }
                }
            }
        },
        confirmButton = {
            TextButton(enabled = text.isNotBlank(), onClick = {
                val c = TEXT_STAMP_COLORS[memory.color]
                onAdd(text, TextLook(memory.size.toDouble(), memory.font, listOf(c.red.toDouble(), c.green.toDouble(), c.blue.toDouble())))
            }) { Text(stringResource(R.string.add)) }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel)) } },
    )
}
