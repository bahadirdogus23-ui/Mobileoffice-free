package org.mobileofficefree.app

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlin.math.max
import kotlin.math.min

/** Excel's sheet size; the grid never scrolls past it. */
private const val MAX_ROWS = 1_048_576
private const val MAX_COLS = 16_384

/**
 * A virtualised spreadsheet grid drawn on one Canvas: only visible cells are measured and drawn,
 * and their texts come from [cellText], which loads them from the engine block by block.
 * Pan with one finger, zoom with two, tap to select.
 */
@Composable
fun SheetGrid(
    sheet: Int,
    usedRows: Int,
    usedCols: Int,
    selectedRow: Int,
    selectedCol: Int,
    cellText: (sheet: Int, row: Int, col: Int) -> String?,
    onSelect: (row: Int, col: Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    val density = LocalDensity.current
    val baseCol = with(density) { 96.dp.toPx() }
    val baseRow = with(density) { 30.dp.toPx() }
    val rowHeader = with(density) { 48.dp.toPx() }
    val colHeader = with(density) { 26.dp.toPx() }
    val measurer = rememberTextMeasurer(cacheSize = 512)
    val colors = MaterialTheme.colorScheme
    val select by rememberUpdatedState(onSelect)

    var scale by remember(sheet) { mutableFloatStateOf(1f) }
    var offsetX by remember(sheet) { mutableFloatStateOf(0f) }
    var offsetY by remember(sheet) { mutableFloatStateOf(0f) }

    // Room to scroll a little past the data, as spreadsheets do.
    val rows = min(MAX_ROWS, max(usedRows + 50, 100))
    val cols = min(MAX_COLS, max(usedCols + 10, 26))

    Canvas(
        modifier
            .fillMaxSize()
            .pointerInput(sheet, rows, cols) {
                detectTransformGestures { centroid, pan, zoom, _ ->
                    val newScale = (scale * zoom).coerceIn(0.4f, 3f)
                    // Keep the content under the fingers still while zooming.
                    val f = newScale / scale
                    val gx = centroid.x - rowHeader
                    val gy = centroid.y - colHeader
                    offsetX = (offsetX + gx) * f - gx - pan.x
                    offsetY = (offsetY + gy) * f - gy - pan.y
                    scale = newScale
                    offsetX = offsetX.coerceIn(0f, max(0f, cols * baseCol * scale - (size.width - rowHeader)))
                    offsetY = offsetY.coerceIn(0f, max(0f, rows * baseRow * scale - (size.height - colHeader)))
                }
            }
            .pointerInput(sheet) {
                detectTapGestures { p ->
                    if (p.x > rowHeader && p.y > colHeader) {
                        val c = ((p.x - rowHeader + offsetX) / (baseCol * scale)).toInt()
                        val r = ((p.y - colHeader + offsetY) / (baseRow * scale)).toInt()
                        select(r.coerceAtMost(MAX_ROWS - 1), c.coerceAtMost(MAX_COLS - 1))
                    }
                }
            },
    ) {
        val cw = baseCol * scale
        val rh = baseRow * scale
        val firstCol = (offsetX / cw).toInt()
        val firstRow = (offsetY / rh).toInt()
        val lastCol = min(cols - 1, ((offsetX + size.width - rowHeader) / cw).toInt())
        val lastRow = min(rows - 1, ((offsetY + size.height - colHeader) / rh).toInt())
        val cellStyle = TextStyle(fontSize = (14 * scale).sp, color = colors.onSurface)
        val headerStyle = TextStyle(fontSize = 12.sp, color = colors.onSurfaceVariant)
        val pad = 4.dp.toPx() * scale

        drawRect(colors.surface)

        // Cells and grid lines.
        for (r in firstRow..lastRow) {
            val y = colHeader + r * rh - offsetY
            drawLine(colors.outlineVariant, Offset(rowHeader, y + rh), Offset(size.width, y + rh), 1f)
            for (c in firstCol..lastCol) {
                val x = rowHeader + c * cw - offsetX
                if (r == firstRow) {
                    drawLine(colors.outlineVariant, Offset(x + cw, colHeader), Offset(x + cw, size.height), 1f)
                }
                val text = cellText(sheet, r, c)
                if (!text.isNullOrEmpty()) {
                    val layout = measurer.measure(
                        text,
                        cellStyle,
                        overflow = TextOverflow.Clip,
                        softWrap = false,
                        maxLines = 1,
                        constraints = Constraints(maxWidth = max(1, (cw - 2 * pad).toInt())),
                    )
                    val numeric = text.first().isDigit() || text.first() == '-' || text.first() == '₺'
                    val tx = if (numeric) x + cw - pad - layout.size.width else x + pad
                    drawText(layout, topLeft = Offset(tx, y + (rh - layout.size.height) / 2))
                }
            }
        }

        // Selection.
        val sx = rowHeader + selectedCol * cw - offsetX
        val sy = colHeader + selectedRow * rh - offsetY
        drawRect(colors.primary, Offset(sx, sy), Size(cw, rh), style = Stroke(2.dp.toPx()))

        // Frozen headers: column letters and row numbers.
        drawRect(colors.surfaceVariant, Offset.Zero, Size(size.width, colHeader))
        drawRect(colors.surfaceVariant, Offset.Zero, Size(rowHeader, size.height))
        for (c in firstCol..lastCol) {
            val x = rowHeader + c * cw - offsetX
            if (c == selectedCol) drawRect(colors.secondaryContainer, Offset(x, 0f), Size(cw, colHeader))
            val l = measurer.measure(columnName(c), headerStyle)
            drawText(l, topLeft = Offset(x + (cw - l.size.width) / 2, (colHeader - l.size.height) / 2))
            drawLine(colors.outline, Offset(x + cw, 0f), Offset(x + cw, colHeader), 1f)
        }
        for (r in firstRow..lastRow) {
            val y = colHeader + r * rh - offsetY
            if (r == selectedRow) drawRect(colors.secondaryContainer, Offset(0f, y), Size(rowHeader, rh))
            val l = measurer.measure((r + 1).toString(), headerStyle)
            drawText(l, topLeft = Offset((rowHeader - l.size.width) / 2, y + (rh - l.size.height) / 2))
            drawLine(colors.outline, Offset(0f, y + rh), Offset(rowHeader, y + rh), 1f)
        }
        drawRect(colors.surfaceVariant, Offset.Zero, Size(rowHeader, colHeader))
    }
}
