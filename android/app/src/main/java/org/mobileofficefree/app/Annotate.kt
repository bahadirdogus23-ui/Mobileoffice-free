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
import org.mobileofficefree.engine.Pen

/** What a touch on a page does while annotating. */
enum class AnnotTool(val label: Int, val pen: Pen?) {
    BLACK(R.string.pen_black, Pen(0.0, 0.0, 0.0, 1.5, 1.0)),
    RED(R.string.pen_red, Pen(0.85, 0.1, 0.1, 1.5, 1.0)),
    BLUE(R.string.pen_blue, Pen(0.1, 0.25, 0.85, 1.5, 1.0)),
    HIGHLIGHT(R.string.highlighter, Pen(1.0, 0.9, 0.0, 12.0, 0.4)),
    NOTE(R.string.note, null),
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
