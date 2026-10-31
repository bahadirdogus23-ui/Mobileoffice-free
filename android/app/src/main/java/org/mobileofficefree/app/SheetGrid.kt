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
import androidx.compose.ui.graphics.drawscope.clipRect
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
    frozenRows: Int = 0,
    frozenCols: Int = 0,
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
    // How far the unfrozen part has scrolled, in pixels.
    var offsetX by remember(sheet) { mutableFloatStateOf(0f) }
    var offsetY by remember(sheet) { mutableFloatStateOf(0f) }

    // Room to scroll a little past the data, as spreadsheets do.
    val rows = min(MAX_ROWS, max(usedRows + 50, 100))
    val cols = min(MAX_COLS, max(usedCols + 10, 26))
    val fr = frozenRows.coerceIn(0, rows - 1)
    val fc = frozenCols.coerceIn(0, cols - 1)

    Canvas(
        modifier
            .fillMaxSize()
            .pointerInput(sheet, rows, cols, columnAxis, rowAxis, fr, fc) {
                detectTransformGestures { centroid, pan, zoom, _ ->
                    val newScale = (scale * zoom).coerceIn(0.4f, 3f)
                    // Keep the content under the fingers still while zooming.
                    val f = newScale / scale
                    val gx = centroid.x - rowHeader
                    val gy = centroid.y - colHeader
                    offsetX = (offsetX + gx) * f - gx - pan.x
                    offsetY = (offsetY + gy) * f - gy - pan.y
                    scale = newScale
                    val k = unit * scale
                    val frozenW = columnAxis.start(fc) * k
                    val frozenH = rowAxis.start(fr) * k
                    offsetX = offsetX.coerceIn(0f, max(0f, (columnAxis.start(cols) - columnAxis.start(fc)) * k - (size.width - rowHeader - frozenW)))
                    offsetY = offsetY.coerceIn(0f, max(0f, (rowAxis.start(rows) - rowAxis.start(fr)) * k - (size.height - colHeader - frozenH)))
                }
            }
            .pointerInput(sheet, columnAxis, rowAxis, fr, fc) {
                detectTapGestures { p ->
                    if (p.x > rowHeader && p.y > colHeader) {
                        val k = unit * scale
                        val gx = p.x - rowHeader
                        val gy = p.y - colHeader
                        val frozenW = columnAxis.start(fc) * k
                        val frozenH = rowAxis.start(fr) * k
                        val c = if (gx < frozenW) columnAxis.at(gx / k) else columnAxis.at(columnAxis.start(fc) + (gx - frozenW + offsetX) / k)
                        val r = if (gy < frozenH) rowAxis.at(gy / k) else rowAxis.at(rowAxis.start(fr) + (gy - frozenH + offsetY) / k)
                        select(r.coerceAtMost(MAX_ROWS - 1), c.coerceAtMost(MAX_COLS - 1))
                    }
                }
            },
    ) {
        val k = unit * scale
        val frozenW = columnAxis.start(fc) * k
        val frozenH = rowAxis.start(fr) * k
        // Where a column or row starts on screen: frozen ones stay put, the others scroll.
        fun xOf(c: Int) = if (c < fc) rowHeader + columnAxis.start(c) * k else rowHeader + frozenW + (columnAxis.start(c) - columnAxis.start(fc)) * k - offsetX
        fun yOf(r: Int) = if (r < fr) colHeader + rowAxis.start(r) * k else colHeader + frozenH + (rowAxis.start(r) - rowAxis.start(fr)) * k - offsetY
        val firstCol = max(fc, columnAxis.at(columnAxis.start(fc) + offsetX / k))
        val firstRow = max(fr, rowAxis.at(rowAxis.start(fr) + offsetY / k))
        val lastCol = min(cols - 1, columnAxis.at(columnAxis.start(fc) + (offsetX + size.width - rowHeader - frozenW) / k))
        val lastRow = min(rows - 1, rowAxis.at(rowAxis.start(fr) + (offsetY + size.height - colHeader - frozenH) / k))
        val scrollCols = (firstCol..lastCol).toList()
        val scrollRows = (firstRow..lastRow).toList()
        val frozenColList = (0 until fc).toList()
        val frozenRowList = (0 until fr).toList()
        val cellStyle = TextStyle(fontSize = (14 * scale).sp, color = colors.onSurface)
        val headerStyle = TextStyle(fontSize = 12.sp, color = colors.onSurfaceVariant)
        val pad = 4.dp.toPx() * scale

        drawRect(colors.surface)

        /** Fills, lines, then texts (a long text may run over the empty cells to its right). */
        fun cells(rowList: List<Int>, colList: List<Int>, left: Float, top: Float, right: Float, bottom: Float) {
            if (rowList.isEmpty() || colList.isEmpty() || right <= left || bottom <= top) return
            clipRect(left, top, right, bottom) {
                for (r in rowList) {
                    val rh = rowAxis.size(r) * k
                    if (rh <= 0f) continue // a hidden row
                    val y = yOf(r)
                    drawLine(colors.outlineVariant, Offset(left, y + rh), Offset(right, y + rh), 1f)
                    for (c in colList) {
                        val cw = columnAxis.size(c) * k
                        if (cw <= 0f) continue // a hidden column
                        val look = cellLook(sheet, r, c) ?: continue
                        if (look.fill != 0u) drawRect(Color(look.fill.toInt()), Offset(xOf(c) + 0.5f, y + 0.5f), Size(cw - 1f, rh - 1f))
                    }
                }
                for (c in colList) {
                    val cw = columnAxis.size(c) * k
                    if (cw <= 0f) continue
                    val x = xOf(c)
                    drawLine(colors.outlineVariant, Offset(x + cw, top), Offset(x + cw, bottom), 1f)
                }
                for (r in rowList) {
                    val rh = rowAxis.size(r) * k
                    if (rh <= 0f) continue
                    val y = yOf(r)
                    for ((i, c) in colList.withIndex()) {
                        val cw = columnAxis.size(c) * k
                        if (cw <= 0f) continue
                        val text = cellText(sheet, r, c)
                        if (text.isNullOrEmpty()) continue
                        val look = cellLook(sheet, r, c)
                        val x = xOf(c)
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
                        // Room to the right: the cell, and the empty cells after it for left-aligned text.
                        var room = cw
                        if (align == 1 || (align == 0 && !numeric)) {
                            var n = i + 1
                            while (n < colList.size && cellText(sheet, r, colList[n]) == "") {
                                room += columnAxis.size(colList[n]) * k
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
            }
        }

        val bodyLeft = rowHeader + frozenW
        val bodyTop = colHeader + frozenH
        cells(scrollRows, scrollCols, bodyLeft, bodyTop, size.width, size.height)
        cells(frozenRowList, scrollCols, bodyLeft, colHeader, size.width, bodyTop)
        cells(scrollRows, frozenColList, rowHeader, bodyTop, bodyLeft, size.height)
        cells(frozenRowList, frozenColList, rowHeader, colHeader, bodyLeft, bodyTop)
        // The edges of frozen panes, as Excel marks them.
        if (fr > 0) drawLine(colors.outline, Offset(rowHeader, bodyTop), Offset(size.width, bodyTop), 2.dp.toPx())
        if (fc > 0) drawLine(colors.outline, Offset(bodyLeft, colHeader), Offset(bodyLeft, size.height), 2.dp.toPx())

        // Selection.
        val selVisible = (selectedCol < fc || selectedCol >= firstCol) && (selectedRow < fr || selectedRow >= firstRow)
        if (selVisible) {
            val sx = xOf(selectedCol)
            val sy = yOf(selectedRow)
            clipRect(if (selectedCol < fc) rowHeader else bodyLeft, if (selectedRow < fr) colHeader else bodyTop, size.width, size.height) {
                drawRect(colors.primary, Offset(sx, sy), Size(columnAxis.size(selectedCol) * k, rowAxis.size(selectedRow) * k), style = Stroke(2.dp.toPx()))
            }
        }

        // Headers: column letters and row numbers.
        drawRect(colors.surfaceVariant, Offset.Zero, Size(size.width, colHeader))
        drawRect(colors.surfaceVariant, Offset.Zero, Size(rowHeader, size.height))
        for (c in frozenColList + scrollCols) {
            val cw = columnAxis.size(c) * k
            if (cw <= 0f) continue
            val x = xOf(c)
            if (c >= fc && x + cw < bodyLeft) continue
            clipRect(if (c < fc) rowHeader else bodyLeft, 0f, size.width, colHeader) {
                if (c == selectedCol) drawRect(colors.secondaryContainer, Offset(x, 0f), Size(cw, colHeader))
                val l = measurer.measure(columnName(c), headerStyle)
                drawText(l, topLeft = Offset(x + (cw - l.size.width) / 2, (colHeader - l.size.height) / 2))
                drawLine(colors.outline, Offset(x + cw, 0f), Offset(x + cw, colHeader), 1f)
            }
        }
        for (r in frozenRowList + scrollRows) {
            val rh = rowAxis.size(r) * k
            if (rh <= 0f) continue
            val y = yOf(r)
            if (r >= fr && y + rh < bodyTop) continue
            clipRect(0f, if (r < fr) colHeader else bodyTop, rowHeader, size.height) {
                if (r == selectedRow) drawRect(colors.secondaryContainer, Offset(0f, y), Size(rowHeader, rh))
                val l = measurer.measure((r + 1).toString(), headerStyle)
                drawText(l, topLeft = Offset((rowHeader - l.size.width) / 2, y + (rh - l.size.height) / 2))
                drawLine(colors.outline, Offset(0f, y + rh), Offset(rowHeader, y + rh), 1f)
            }
        }
        drawRect(colors.surfaceVariant, Offset.Zero, Size(rowHeader, colHeader))
    }
}
