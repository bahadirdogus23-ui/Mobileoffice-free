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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.mobileofficefree.engine.CellLook
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
    cellLook: (sheet: Int, row: Int, col: Int) -> CellLook?,
    columnAxis: GridAxis,
    rowAxis: GridAxis,
    onSelect: (row: Int, col: Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    val density = LocalDensity.current
    // Pixels per point at zoom 1: the file's sizes, a little larger than on paper for fingers.
    val unit = with(density) { 1.7.dp.toPx() }
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
            .pointerInput(sheet, rows, cols, columnAxis, rowAxis) {
                detectTransformGestures { centroid, pan, zoom, _ ->
                    val newScale = (scale * zoom).coerceIn(0.4f, 3f)
                    // Keep the content under the fingers still while zooming.
                    val f = newScale / scale
                    val gx = centroid.x - rowHeader
                    val gy = centroid.y - colHeader
                    offsetX = (offsetX + gx) * f - gx - pan.x
                    offsetY = (offsetY + gy) * f - gy - pan.y
                    scale = newScale
                    offsetX = offsetX.coerceIn(0f, max(0f, columnAxis.start(cols) * unit * scale - (size.width - rowHeader)))
                    offsetY = offsetY.coerceIn(0f, max(0f, rowAxis.start(rows) * unit * scale - (size.height - colHeader)))
                }
            }
            .pointerInput(sheet, columnAxis, rowAxis) {
                detectTapGestures { p ->
                    if (p.x > rowHeader && p.y > colHeader) {
                        val c = columnAxis.at((p.x - rowHeader + offsetX) / (unit * scale))
                        val r = rowAxis.at((p.y - colHeader + offsetY) / (unit * scale))
                        select(r.coerceAtMost(MAX_ROWS - 1), c.coerceAtMost(MAX_COLS - 1))
                    }
                }
            },
    ) {
        val k = unit * scale
        val firstCol = columnAxis.at(offsetX / k)
        val firstRow = rowAxis.at(offsetY / k)
        val lastCol = min(cols - 1, columnAxis.at((offsetX + size.width - rowHeader) / k))
        val lastRow = min(rows - 1, rowAxis.at((offsetY + size.height - colHeader) / k))
        val cellStyle = TextStyle(fontSize = (14 * scale).sp, color = colors.onSurface)
        val headerStyle = TextStyle(fontSize = 12.sp, color = colors.onSurfaceVariant)
        val pad = 4.dp.toPx() * scale

        drawRect(colors.surface)

        // Fills and row lines first, then column lines, then the texts on top (a long text may
        // run over the empty cells to its right, as in Excel, and must not be cut by their lines).
        for (r in firstRow..lastRow) {
            val rh = rowAxis.size(r) * k
            if (rh <= 0f) continue // a hidden row
            val y = colHeader + rowAxis.start(r) * k - offsetY
            drawLine(colors.outlineVariant, Offset(rowHeader, y + rh), Offset(size.width, y + rh), 1f)
            for (c in firstCol..lastCol) {
                val cw = columnAxis.size(c) * k
                if (cw <= 0f) continue // a hidden column
                val look = cellLook(sheet, r, c) ?: continue
                if (look.fill != 0u) {
                    val x = rowHeader + columnAxis.start(c) * k - offsetX
                    drawRect(Color(look.fill.toInt()), Offset(x + 0.5f, y + 0.5f), Size(cw - 1f, rh - 1f))
                }
            }
        }
        for (c in firstCol..lastCol) {
            val cw = columnAxis.size(c) * k
            if (cw <= 0f) continue
            val x = rowHeader + columnAxis.start(c) * k - offsetX
            drawLine(colors.outlineVariant, Offset(x + cw, colHeader), Offset(x + cw, size.height), 1f)
        }
        for (r in firstRow..lastRow) {
            val rh = rowAxis.size(r) * k
            if (rh <= 0f) continue
            val y = colHeader + rowAxis.start(r) * k - offsetY
            for (c in firstCol..lastCol) {
                val cw = columnAxis.size(c) * k
                if (cw <= 0f) continue
                val text = cellText(sheet, r, c)
                if (text.isNullOrEmpty()) continue
                val look = cellLook(sheet, r, c)
                val x = rowHeader + columnAxis.start(c) * k - offsetX
                val style = if (look == null) cellStyle else cellStyle.copy(
                    fontWeight = if (look.bold) FontWeight.Bold else null,
                    fontStyle = if (look.italic) FontStyle.Italic else null,
                    textDecoration = if (look.underline) TextDecoration.Underline else null,
                    // Text keeps its own colour; on a filled cell the default is black, as on paper.
                    color = when {
                        look.color != 0u -> Color(look.color.toInt())
                        look.fill != 0u -> Color.Black
                        else -> cellStyle.color
                    },
                )
                val numeric = text.first().isDigit() || text.first() == '-' || text.first() == '₺'
                val align = look?.align?.toInt() ?: 0
                val leftAligned = align == 1 || (align == 0 && !numeric)
                // Room to the right: the cell, and the empty cells after it for left-aligned text.
                var room = cw
                if (leftAligned) {
                    var n = c + 1
                    while (n <= lastCol && cellText(sheet, r, n) == "") {
                        room += columnAxis.size(n) * k
                        n++
                    }
                }
                val layout = measurer.measure(
                    text,
                    style,
                    overflow = TextOverflow.Clip,
                    softWrap = false,
                    maxLines = 1,
                    constraints = Constraints(maxWidth = max(1, (room - 2 * pad).toInt())),
                )
                val tx = when {
                    align == 2 -> x + (cw - layout.size.width) / 2
                    align == 3 || (align == 0 && numeric) -> x + cw - pad - layout.size.width
                    else -> x + pad
                }
                drawText(layout, topLeft = Offset(tx, y + (rh - layout.size.height) / 2))
            }
        }

        // Selection.
        val sx = rowHeader + columnAxis.start(selectedCol) * k - offsetX
        val sy = colHeader + rowAxis.start(selectedRow) * k - offsetY
        drawRect(colors.primary, Offset(sx, sy), Size(columnAxis.size(selectedCol) * k, rowAxis.size(selectedRow) * k), style = Stroke(2.dp.toPx()))

        // Frozen headers: column letters and row numbers.
        drawRect(colors.surfaceVariant, Offset.Zero, Size(size.width, colHeader))
        drawRect(colors.surfaceVariant, Offset.Zero, Size(rowHeader, size.height))
        for (c in firstCol..lastCol) {
            val cw = columnAxis.size(c) * k
            if (cw <= 0f) continue
            val x = rowHeader + columnAxis.start(c) * k - offsetX
            if (c == selectedCol) drawRect(colors.secondaryContainer, Offset(x, 0f), Size(cw, colHeader))
            val l = measurer.measure(columnName(c), headerStyle)
            drawText(l, topLeft = Offset(x + (cw - l.size.width) / 2, (colHeader - l.size.height) / 2))
            drawLine(colors.outline, Offset(x + cw, 0f), Offset(x + cw, colHeader), 1f)
        }
        for (r in firstRow..lastRow) {
            val rh = rowAxis.size(r) * k
            if (rh <= 0f) continue
            val y = colHeader + rowAxis.start(r) * k - offsetY
            if (r == selectedRow) drawRect(colors.secondaryContainer, Offset(0f, y), Size(rowHeader, rh))
            val l = measurer.measure((r + 1).toString(), headerStyle)
            drawText(l, topLeft = Offset((rowHeader - l.size.width) / 2, y + (rh - l.size.height) / 2))
            drawLine(colors.outline, Offset(0f, y + rh), Offset(rowHeader, y + rh), 1f)
        }
        drawRect(colors.surfaceVariant, Offset.Zero, Size(rowHeader, colHeader))
    }
}
