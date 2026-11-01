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
import androidx.compose.ui.graphics.drawscope.DrawScope
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
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.mobileofficefree.engine.CellLook
import kotlin.math.max
import kotlin.math.roundToInt
import androidx.compose.ui.unit.IntOffset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.gestures.detectDragGesturesAfterLongPress
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
    /** Merged ranges: [first row, first column, last row, last column]. */
    merges: List<IntArray> = emptyList(),
    /** The far corner of the selected range (the selected cell when one is chosen). */
    endRow: Int = selectedRow,
    endCol: Int = selectedCol,
    /** A long press and drag stretches the selection to this cell. */
    onExtend: (row: Int, col: Int) -> Unit = { _, _ -> },
    /** A tap on a row number or a column letter: the whole row or column. */
    onLine: (row: Int?, col: Int?) -> Unit = { _, _ -> },
    /** A tap on the cell that is already selected: type in it. */
    onEditCell: () -> Unit = {},
    /** Shown over the selected cell while it is typed in. */
    cellEditor: (@Composable (Modifier) -> Unit)? = null,
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

    val extend by rememberUpdatedState(onExtend)
    val line by rememberUpdatedState(onLine)
    val edit by rememberUpdatedState(onEditCell)
    /** The cell under screen point [p] (inside the cells area). */
    fun cellAt(p: Offset): Pair<Int, Int> {
        val k = unit * scale
        val gx = p.x - rowHeader
        val gy = p.y - colHeader
        val frozenW = columnAxis.start(fc) * k
        val frozenH = rowAxis.start(fr) * k
        val c = if (gx < frozenW) columnAxis.at(gx / k) else columnAxis.at(columnAxis.start(fc) + (gx - frozenW + offsetX) / k)
        val r = if (gy < frozenH) rowAxis.at(gy / k) else rowAxis.at(rowAxis.start(fr) + (gy - frozenH + offsetY) / k)
        return r.coerceIn(0, MAX_ROWS - 1) to c.coerceIn(0, MAX_COLS - 1)
    }

    Box(modifier.fillMaxSize()) {
    Canvas(
        Modifier
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
                    when {
                        p.x > rowHeader && p.y > colHeader -> {
                            val (r, c) = cellAt(p)
                            if (r == selectedRow && c == selectedCol && endRow == selectedRow && endCol == selectedCol) edit() else select(r, c)
                        }
                        p.y <= colHeader && p.x > rowHeader -> line(null, cellAt(Offset(p.x, colHeader + 1f)).second)
                        p.x <= rowHeader && p.y > colHeader -> line(cellAt(Offset(rowHeader + 1f, p.y)).first, null)
                    }
                }
            }
            .pointerInput(sheet, columnAxis, rowAxis, fr, fc) {
                // Hold a cell, then drag: the selection stretches with the finger.
                detectDragGesturesAfterLongPress(
                    onDragStart = { p -> if (p.x > rowHeader && p.y > colHeader) cellAt(p).let { (r, c) -> select(r, c) } },
                ) { change, _ ->
                    change.consume()
                    val p = change.position
                    if (p.x > rowHeader && p.y > colHeader) cellAt(p).let { (r, c) -> extend(r, c) }
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
        // The cells are paper, as in the file: white, black text, Excel's light grey grid, in the
        // dark theme too (the file's own colours are drawn as they are).
        val paper = Color.White
        val gridLine = Color(0xFFD9D9D9)
        val cellStyle = TextStyle(fontSize = (14 * scale).sp, color = Color.Black)
        val headerStyle = TextStyle(fontSize = 12.sp, color = colors.onSurfaceVariant)
        val pad = 4.dp.toPx() * scale

        drawRect(paper)

        /** [text] in a box at ([x], [y]) [w]×[h], aligned by [look], given [room] to run to the right. */
        fun DrawScope.drawCell(text: String, look: CellLook?, x: Float, y: Float, w: Float, h: Float, room: Float) {
            val style = if (look == null) cellStyle else cellStyle.copy(
                fontWeight = if (look.bold) FontWeight.Bold else null,
                fontStyle = if (look.italic) FontStyle.Italic else null,
                textDecoration = if (look.underline) TextDecoration.Underline else null,
                // Text keeps its own colour; on a filled cell the default is black, as on paper.
                color = when {
                    look.color != 0u -> Color(look.color.toInt())
                    else -> cellStyle.color
                },
            )
            val numeric = text.first().isDigit() || text.first() == '-' || text.first() == '₺'
            val align = look?.align?.toInt() ?: 0
            // Text that wraps (or has its own line breaks) stays inside its cell, line by line;
            // other text is one line that may run on over empty cells.
            val wraps = look?.wrap == true || '\n' in text
            val lineAlign = when {
                align == 2 -> TextAlign.Center
                align == 3 || (align == 0 && numeric) -> TextAlign.End
                else -> TextAlign.Start
            }
            val box = max(1, ((if (wraps) w else room) - 2 * pad).toInt())
            val layout = measurer.measure(
                text,
                style.copy(textAlign = lineAlign),
                overflow = TextOverflow.Clip,
                softWrap = look?.wrap == true,
                maxLines = if (wraps) Int.MAX_VALUE else 1,
                constraints = Constraints(minWidth = if (wraps) box else 0, maxWidth = box),
            )
            val tx = when {
                wraps -> x + pad
                align == 2 -> x + (w - layout.size.width) / 2
                align == 3 || (align == 0 && numeric) -> x + w - pad - layout.size.width
                else -> x + pad
            }
            // Excel puts text at the bottom unless the cell says centre or top.
            val ty = when (look?.valign?.toInt() ?: 0) {
                2 -> y + pad / 2
                1 -> y + (h - layout.size.height) / 2
                else -> y + h - layout.size.height - pad / 2
            }
            if (wraps) {
                clipRect(x, y, x + w, y + h) { drawText(layout, topLeft = Offset(tx, max(y, ty))) }
            } else {
                drawText(layout, topLeft = Offset(tx, ty))
            }
        }

        /** Fills, lines, then texts (a long text may run over the empty cells to its right). */
        fun cells(rowList: List<Int>, colList: List<Int>, left: Float, top: Float, right: Float, bottom: Float) {
            if (rowList.isEmpty() || colList.isEmpty() || right <= left || bottom <= top) return
            clipRect(left, top, right, bottom) {
                for (r in rowList) {
                    val rh = rowAxis.size(r) * k
                    if (rh <= 0f) continue // a hidden row
                    val y = yOf(r)
                    drawLine(gridLine, Offset(left, y + rh), Offset(right, y + rh), 1f)
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
                    drawLine(gridLine, Offset(x + cw, top), Offset(x + cw, bottom), 1f)
                }
                // Merged ranges: one box (its first cell's fill, or paper over the inner lines);
                // the first cell's text goes across all of it below.
                val shown = merges.filter { m -> m[2] >= rowList.first() && m[0] <= rowList.last() && m[3] >= colList.first() && m[1] <= colList.last() }
                for (m in shown) {
                    val x0 = xOf(m[1])
                    val y0 = yOf(m[0])
                    val w = (columnAxis.start(m[3] + 1) - columnAxis.start(m[1])) * k
                    val h = (rowAxis.start(m[2] + 1) - rowAxis.start(m[0])) * k
                    val look = cellLook(sheet, m[0], m[1])
                    drawRect(if (look != null && look.fill != 0u) Color(look.fill.toInt()) else paper, Offset(x0 + 0.5f, y0 + 0.5f), Size(w - 1f, h - 1f))
                }
                fun inside(r: Int, c: Int) = shown.firstOrNull { m -> r in m[0]..m[2] && c in m[1]..m[3] }
                // Cell borders from the file, over the grid lines and merged boxes: thin, medium
                // and thick as on paper (¾, 1½ and 2¼ points), never under one pixel. A side inside
                // a merged range is not drawn.
                for (r in rowList) {
                    val rh = rowAxis.size(r) * k
                    if (rh <= 0f) continue
                    val y = yOf(r)
                    for (c in colList) {
                        val cw = columnAxis.size(c) * k
                        if (cw <= 0f) continue
                        val look = cellLook(sheet, r, c) ?: continue
                        val b = look.borders.toInt()
                        if (b == 0) continue
                        val color = if (look.borderColor != 0u) Color(look.borderColor.toInt()) else Color.Black
                        val x = xOf(c)
                        val m = inside(r, c)
                        fun side(shift: Int, inner: Boolean, from: Offset, to: Offset) {
                            val w = (b shr shift) and 0xFF
                            if (w > 0 && !inner) drawLine(color, from, to, max(1f, w * 0.75f * k))
                        }
                        side(0, m != null && c > m[1], Offset(x, y), Offset(x, y + rh))
                        side(8, m != null && c < m[3], Offset(x + cw, y), Offset(x + cw, y + rh))
                        side(16, m != null && r > m[0], Offset(x, y), Offset(x + cw, y))
                        side(24, m != null && r < m[2], Offset(x, y + rh), Offset(x + cw, y + rh))
                    }
                }
                for (m in shown) {
                    val text = cellText(sheet, m[0], m[1])
                    if (text.isNullOrEmpty()) continue
                    val w = (columnAxis.start(m[3] + 1) - columnAxis.start(m[1])) * k
                    val h = (rowAxis.start(m[2] + 1) - rowAxis.start(m[0])) * k
                    drawCell(text, cellLook(sheet, m[0], m[1]), xOf(m[1]), yOf(m[0]), w, h, w)
                }
                for (r in rowList) {
                    val rh = rowAxis.size(r) * k
                    if (rh <= 0f) continue
                    val y = yOf(r)
                    for ((i, c) in colList.withIndex()) {
                        val cw = columnAxis.size(c) * k
                        if (cw <= 0f) continue
                        if (shown.any { m -> r in m[0]..m[2] && c in m[1]..m[3] }) continue
                        val text = cellText(sheet, r, c)
                        if (text.isNullOrEmpty()) continue
                        val look = cellLook(sheet, r, c)
                        val numeric = text.first().isDigit() || text.first() == '-' || text.first() == '₺'
                        val align = look?.align?.toInt() ?: 0
                        // Room to the right: the cell, and the empty cells after it for left-aligned text.
                        var room = cw
                        if (align == 1 || (align == 0 && !numeric)) {
                            var n = i + 1
                            while (n < colList.size && cellText(sheet, r, colList[n]) == "" && shown.none { m -> r in m[0]..m[2] && colList[n] in m[1]..m[3] }) {
                                room += columnAxis.size(colList[n]) * k
                                n++
                            }
                        }
                        drawCell(text, look, xOf(c), y, cw, rh, room)
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

        // Selection: the range tinted and framed, its first cell framed.
        val r0 = min(selectedRow, endRow)
        val r1 = max(selectedRow, endRow)
        val c0 = min(selectedCol, endCol)
        val c1 = max(selectedCol, endCol)
        clipRect(rowHeader, colHeader, size.width, size.height) {
            val x0 = xOf(c0)
            val y0 = yOf(r0)
            val w = (columnAxis.start(c1 + 1) - columnAxis.start(c0)) * k
            val h = (rowAxis.start(r1 + 1) - rowAxis.start(r0)) * k
            if (r0 != r1 || c0 != c1) drawRect(colors.primary.copy(alpha = 0.12f), Offset(x0, y0), Size(w, h))
            drawRect(colors.primary, Offset(x0, y0), Size(w, h), style = Stroke(2.dp.toPx()))
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
                if (c in c0..c1) drawRect(colors.secondaryContainer, Offset(x, 0f), Size(cw, colHeader))
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
                if (r in r0..r1) drawRect(colors.secondaryContainer, Offset(0f, y), Size(rowHeader, rh))
                val l = measurer.measure((r + 1).toString(), headerStyle)
                drawText(l, topLeft = Offset((rowHeader - l.size.width) / 2, y + (rh - l.size.height) / 2))
                drawLine(colors.outline, Offset(0f, y + rh), Offset(rowHeader, y + rh), 1f)
            }
        }
        drawRect(colors.surfaceVariant, Offset.Zero, Size(rowHeader, colHeader))
    }
    // Typing in the cell: a field laid exactly over it (at least a comfortable size).
    if (cellEditor != null) {
        val k = unit * scale
        val frozenW = columnAxis.start(fc) * k
        val frozenH = rowAxis.start(fr) * k
        val x = if (selectedCol < fc) rowHeader + columnAxis.start(selectedCol) * k else rowHeader + frozenW + (columnAxis.start(selectedCol) - columnAxis.start(fc)) * k - offsetX
        val y = if (selectedRow < fr) colHeader + rowAxis.start(selectedRow) * k else colHeader + frozenH + (rowAxis.start(selectedRow) - rowAxis.start(fr)) * k - offsetY
        val w = max(columnAxis.size(selectedCol) * k, with(density) { 120.dp.toPx() })
        val h = max(rowAxis.size(selectedRow) * k, with(density) { 40.dp.toPx() })
        cellEditor(
            Modifier
                .offset { IntOffset(max(x, rowHeader).roundToInt(), max(y, colHeader).roundToInt()) }
                .size(with(density) { w.toDp() }, with(density) { h.toDp() }),
        )
    }
    }
}
