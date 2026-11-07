package org.mobileofficefree.app

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import kotlin.math.roundToInt
import org.mobileofficefree.engine.PageRect

/**
 * Android's selection handles at both ends of a PDF text selection on page [page]: dragging one
 * moves that end while the other stays. [rects] are the selected lines (fractions of the page).
 */
@Composable
fun PdfSelectionHandles(pdf: PdfViewModel, page: Int, rects: List<PageRect>) {
    if (rects.isEmpty()) return
    val color = MaterialTheme.colorScheme.primary
    val handle = 22.dp
    val handlePx = with(LocalDensity.current) { handle.toPx() }
    val first = rects.first()
    val last = rects.last()
    BoxWithConstraints(Modifier.fillMaxSize()) {
        val w = constraints.maxWidth.toFloat()
        val h = constraints.maxHeight.toFloat()
        // (is the start handle, the line it moves, the line that stays)
        for ((start, moving, fixed) in listOf(Triple(true, first, last), Triple(false, last, first))) {
            key(start) {
                var drag by remember { mutableStateOf<Offset?>(null) }
                var from by remember { mutableStateOf(Offset.Zero) }
                val movingNow by rememberUpdatedState(moving)
                val fixedNow by rememberUpdatedState(fixed)
                val atX = (if (start) moving.left else moving.right).toFloat() * w
                val atY = moving.bottom.toFloat() * h
                val d = drag
                val x = (if (d != null) from.x + d.x else atX) - (if (start) handlePx else 0f)
                val y = if (d != null) from.y + d.y else atY
                Box(
                    Modifier
                        .offset { IntOffset(x.roundToInt(), y.roundToInt()) }
                        .size(handle)
                        .background(color, if (start) RoundedCornerShape(50, 0, 50, 50) else RoundedCornerShape(0, 50, 50, 50))
                        .pointerInput(start) {
                            detectDragGestures(
                                onDragStart = {
                                    val m = movingNow
                                    from = Offset((if (start) m.left else m.right).toFloat() * w, m.bottom.toFloat() * h)
                                    drag = Offset.Zero
                                },
                                onDragEnd = { drag = null },
                                onDragCancel = { drag = null },
                            ) { change, amount ->
                                change.consume()
                                val now = (drag ?: return@detectDragGestures) + amount
                                drag = now
                                val f = fixedNow
                                val m = movingNow
                                // The end that stays, and the line just above the handle where the finger points.
                                val fx = (if (start) f.right else f.left).toFloat()
                                val fy = ((f.top + f.bottom) / 2).toFloat()
                                val lineH = (m.bottom - m.top).toFloat() * h
                                pdf.selectFromFixed(page, fx, fy, (from.x + now.x) / w, (from.y + now.y - lineH / 2) / h)
                            }
                        },
                )
            }
        }
    }
}
