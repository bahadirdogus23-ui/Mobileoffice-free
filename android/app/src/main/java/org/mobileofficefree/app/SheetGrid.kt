package org.mobileofficefree.app

import androidx.compose.ui.graphics.asImageBitmap
import kotlin.math.abs
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.LaunchedEffect
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
import androidx.compose.runtime.key
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.CollectionInfo
import androidx.compose.ui.semantics.CollectionItemInfo
import androidx.compose.ui.semantics.ScrollAxisRange
import androidx.compose.ui.semantics.collectionInfo
import androidx.compose.ui.semantics.collectionItemInfo
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.horizontalScrollAxisRange
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.scrollBy
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.verticalScrollAxisRange
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
/** At most this many cells get a node of their own for a screen reader (the sheet zoomed far out). */
private const val MAX_SPOKEN_CELLS = 300

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
    /** Grows when the selected cell must be scrolled into view (a search result). */
    reveal: Int = 0,
    /** The sheet's pictures and charts, each with its bitmap once ready (null while it is being made). */
    charts: List<Pair<org.mobileofficefree.engine.SheetChartBox, android.graphics.Bitmap?>> = emptyList(),
    /** Cells with a note or comment, as row × 16384 + column: marked with Excel's red corner. */
    noteCells: Set<Long> = emptySet(),
    /** A column ([col] ≥ 0) or row dragged to a new size (the axes' unit) from its header's edge. */
    onResize: (col: Int, row: Int, points: Float) -> Unit = { _, _, _ -> },
    /** A tap on chart [index] (the sheet's chart index; pictures do not count). */
    onChartTap: (index: Int) -> Unit = {},
    /** The chart index shown as picked (framed), if any. */
    pickedChart: Int? = null,
    /** A whole row or column chosen by its header: merged cells do not widen the selection. */
    wholeLine: Boolean = false,
    /** The ranges a formula being typed refers to ([first row, first column, last row, last column]), framed in colour as Excel does. */
    formulaRefs: List<IntArray> = emptyList(),
) {
    // A column or row being dragged wider or narrower: (is a column, its index, its size in points).
    var resizing by remember(sheet) { mutableStateOf<Triple<Boolean, Int, Float>?>(null) }
    val baseCols by rememberUpdatedState(columnAxis)
    val baseRows by rememberUpdatedState(rowAxis)
    val resized by rememberUpdatedState(onResize)
    @Suppress("NAME_SHADOWING")
    val columnAxis = resizing?.takeIf { it.first }?.let { columnAxis.with(it.second, it.third) } ?: columnAxis
    @Suppress("NAME_SHADOWING")
    val rowAxis = resizing?.takeIf { !it.first }?.let { rowAxis.with(it.second, it.third) } ?: rowAxis
    val density = LocalDensity.current
    // Pixels per point at zoom 1: the file's sizes, a little larger than on paper for fingers.
    val unit = with(density) { 1.7.dp.toPx() }
    val rowHeader = with(density) { 48.dp.toPx() }
    val colHeader = with(density) { 26.dp.toPx() }
    val measurer = rememberTextMeasurer(cacheSize = 512)
    val colors = MaterialTheme.colorScheme
    val select by rememberUpdatedState(onSelect)
    val screenReader = rememberScreenReaderOn()

    var scale by remember(sheet) { mutableFloatStateOf(1f) }
    // How far the unfrozen part has scrolled, in pixels.
    var offsetX by remember(sheet) { mutableFloatStateOf(0f) }
    var offsetY by remember(sheet) { mutableFloatStateOf(0f) }

    // Room to scroll a little past the data, as spreadsheets do.
    val rows = min(MAX_ROWS, max(max(usedRows + 50, max(selectedRow, endRow) + 51), 100))
    val cols = min(MAX_COLS, max(max(usedCols + 10, max(selectedCol, endCol) + 11), 26))
    val rowsNow by rememberUpdatedState(rows)
    val colsNow by rememberUpdatedState(cols)
    val fr = frozenRows.coerceIn(0, rows - 1)
    val fc = frozenCols.coerceIn(0, cols - 1)

    // The selection and merges as they are now: the gesture handlers below outlive recompositions,
    // so they must not keep the values from when they started.
    val selNow by rememberUpdatedState(intArrayOf(selectedRow, selectedCol, endRow, endCol))
    val mergesNow by rememberUpdatedState(merges)
    /** The merged range covering (`r`, `c`), if any: [first row, first column, last row, last column]. */
    fun mergeAt(r: Int, c: Int): IntArray? = mergesNow.firstOrNull { r in it[0]..it[2] && c in it[1]..it[3] }
    /** The cell a tap on (`r`, `c`) means: a merged range's first cell, where its value lives. */
    fun anchorOf(r: Int, c: Int): Pair<Int, Int> = mergeAt(r, c)?.let { it[0] to it[1] } ?: (r to c)
    val extend by rememberUpdatedState(onExtend)
    val chartsNow by rememberUpdatedState(charts)
    val chartTap by rememberUpdatedState(onChartTap)
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

    var viewSize by remember { mutableStateOf(androidx.compose.ui.unit.IntSize.Zero) }
    // Brings the selected cell into view, keeping a little margin around it.
    LaunchedEffect(reveal) {
        if (reveal == 0 || viewSize.width == 0) return@LaunchedEffect
        val k = unit * scale
        val frozenW = columnAxis.start(fc) * k
        val frozenH = rowAxis.start(fr) * k
        val roomW = viewSize.width - rowHeader - frozenW
        val roomH = viewSize.height - colHeader - frozenH
        if (selectedCol >= fc) {
            val x0 = (columnAxis.start(selectedCol) - columnAxis.start(fc)) * k
            val x1 = (columnAxis.start(selectedCol + 1) - columnAxis.start(fc)) * k
            if (x0 < offsetX || x1 > offsetX + roomW) offsetX = max(0f, x0 - roomW / 3)
        }
        if (selectedRow >= fr) {
            val y0 = (rowAxis.start(selectedRow) - rowAxis.start(fr)) * k
            val y1 = (rowAxis.start(selectedRow + 1) - rowAxis.start(fr)) * k
            if (y0 < offsetY || y1 > offsetY + roomH) offsetY = max(0f, y0 - roomH / 3)
        }
    }

    // Stretching a selection near an edge: the sheet scrolls on by itself, as in Excel, for as
    // long as the finger stays there.
    var dragPoint by remember(sheet) { mutableStateOf<Offset?>(null) }
    LaunchedEffect(dragPoint != null) {
        while (true) {
            val p = dragPoint ?: break
            if (viewSize.width <= rowHeader + 1f || viewSize.height <= colHeader + 1f) {
                kotlinx.coroutines.delay(16)
                continue
            }
            val zone = with(density) { 40.dp.toPx() }
            fun speed(depth: Float) = (depth / zone).coerceIn(0f, 1f) * with(density) { 14.dp.toPx() }
            val vx = speed(p.x - (viewSize.width - zone)) - speed(rowHeader + zone - p.x)
            val vy = speed(p.y - (viewSize.height - zone)) - speed(colHeader + zone - p.y)
            if (vx != 0f || vy != 0f) {
                val k = unit * scale
                val frozenW = columnAxis.start(fc) * k
                val frozenH = rowAxis.start(fr) * k
                offsetX = (offsetX + vx).coerceIn(0f, max(0f, (columnAxis.start(colsNow) - columnAxis.start(fc)) * k - (viewSize.width - rowHeader - frozenW)))
                offsetY = (offsetY + vy).coerceIn(0f, max(0f, (rowAxis.start(rowsNow) - rowAxis.start(fr)) * k - (viewSize.height - colHeader - frozenH)))
                val inside = Offset(p.x.coerceIn(rowHeader + 1f, viewSize.width - 1f), p.y.coerceIn(colHeader + 1f, viewSize.height - 1f))
                cellAt(inside).let { (r, c) -> extend(r, c) }
            }
            kotlinx.coroutines.delay(16)
        }
    }

    // A screen reader cannot see the drawn cells: name the grid and the selected cell with its text.
    val gridLabel = stringResource(
        R.string.a11y_sheet_grid,
        columnName(selectedCol) + (selectedRow + 1),
        cellText(sheet, selectedRow, selectedCol).orEmpty(),
    )
    Box(modifier.fillMaxSize().onSizeChanged { viewSize = it }) {
    Canvas(
        Modifier
            .fillMaxSize()
            .semantics { contentDescription = gridLabel }
            .pointerInput(sheet, fr, fc) {
                // Dragging the edge between two column letters (or row numbers) sizes that column (row).
                val edge = 14.dp.toPx()
                awaitEachGesture {
                    val down = awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
                    val p = down.position
                    val k = unit * scale
                    val cAxis = baseCols
                    val rAxis = baseRows
                    val target = when {
                        p.y <= colHeader && p.x > rowHeader -> {
                            // The column whose right edge is under the finger.
                            val frozenW = cAxis.start(fc) * k
                            fun rightEdge(c: Int) = if (c < fc) rowHeader + cAxis.start(c + 1) * k else rowHeader + frozenW + (cAxis.start(c + 1) - cAxis.start(fc)) * k - offsetX
                            val gx = p.x - rowHeader
                            val at = if (gx < frozenW) cAxis.at(gx / k) else cAxis.at(cAxis.start(fc) + (gx - frozenW + offsetX) / k)
                            listOf(at, at - 1).firstOrNull { it >= 0 && abs(rightEdge(it) - p.x) < edge }?.let { Triple(true, it, cAxis.size(it)) }
                        }
                        p.x <= rowHeader && p.y > colHeader -> {
                            val frozenH = rAxis.start(fr) * k
                            fun bottomEdge(r: Int) = if (r < fr) colHeader + rAxis.start(r + 1) * k else colHeader + frozenH + (rAxis.start(r + 1) - rAxis.start(fr)) * k - offsetY
                            val gy = p.y - colHeader
                            val at = if (gy < frozenH) rAxis.at(gy / k) else rAxis.at(rAxis.start(fr) + (gy - frozenH + offsetY) / k)
                            listOf(at, at - 1).firstOrNull { it >= 0 && abs(bottomEdge(it) - p.y) < edge }?.let { Triple(false, it, rAxis.size(it)) }
                        }
                        else -> null
                    } ?: return@awaitEachGesture
                    down.consume()
                    resizing = target
                    var size = target.third
                    while (true) {
                        val event = awaitPointerEvent(PointerEventPass.Initial)
                        val change = event.changes.firstOrNull { it.id == down.id } ?: break
                        if (!change.pressed) break
                        val d = change.position - change.previousPosition
                        size = (size + (if (target.first) d.x else d.y) / k).coerceIn(2f, 1000f)
                        resizing = Triple(target.first, target.second, size)
                        change.consume()
                    }
                    resizing = null
                    if (size != target.third) resized(if (target.first) target.second else -1, if (target.first) -1 else target.second, size)
                }
            }
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
                    // A chart under the finger (the topmost): picked rather than the cell beneath.
                    val k = unit * scale
                    val frozenW = columnAxis.start(fc) * k
                    val frozenH = rowAxis.start(fr) * k
                    fun sx(c: Int) = if (c < fc) rowHeader + columnAxis.start(c) * k else rowHeader + frozenW + (columnAxis.start(c) - columnAxis.start(fc)) * k - offsetX
                    fun sy(r: Int) = if (r < fr) colHeader + rowAxis.start(r) * k else colHeader + frozenH + (rowAxis.start(r) - rowAxis.start(fr)) * k - offsetY
                    val chart = chartsNow.lastOrNull { (box, _) ->
                        // Charts only: pictures have no menu yet. A chart's picture slot is made by chartPicture.
                        box.isChart && run {
                            val x = sx(box.col.toInt()) + box.dx * k
                            val y = sy(box.row.toInt()) + box.dy * k
                            p.x in x..(x + box.width * k) && p.y in y..(y + box.height * k)
                        }
                    }
                    if (chart != null && p.x > rowHeader && p.y > colHeader) {
                        chartTap(chart.first.index.toInt())
                        return@detectTapGestures
                    }
                    when {
                        p.x > rowHeader && p.y > colHeader -> {
                            val (r, c) = cellAt(p).let { (r, c) -> anchorOf(r, c) }
                            val now = selNow
                            if (r == now[0] && c == now[1] && now[2] == now[0] && now[3] == now[1]) edit() else select(r, c)
                        }
                        p.y <= colHeader && p.x > rowHeader -> line(null, cellAt(Offset(p.x, colHeader + 1f)).second)
                        p.x <= rowHeader && p.y > colHeader -> line(cellAt(Offset(rowHeader + 1f, p.y)).first, null)
                    }
                }
            }
            .pointerInput(sheet, columnAxis, rowAxis, fr, fc) {
                // Hold a cell, then drag: the selection stretches with the finger.
                detectDragGesturesAfterLongPress(
                    onDragStart = { p ->
                        if (p.x > rowHeader && p.y > colHeader) {
                            cellAt(p).let { (r, c) -> anchorOf(r, c) }.let { (r, c) -> select(r, c) }
                            dragPoint = p
                        }
                    },
                    onDragEnd = { dragPoint = null },
                    onDragCancel = { dragPoint = null },
                ) { change, _ ->
                    change.consume()
                    val p = change.position
                    if (dragPoint != null) dragPoint = p
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
            val align = look?.align?.toInt() ?: 0
            // Text that wraps (or has its own line breaks) stays inside its cell, line by line;
            // other text is one line that may run on over empty cells.
            val wraps = look?.wrap == true || '\n' in text
            val lineAlign = when {
                align == 2 -> TextAlign.Center
                align == 3 -> TextAlign.End
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
                align == 3 -> x + w - pad - layout.size.width
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
                        // A conditional format's data bar, under the value as Excel draws it.
                        if (look.bar > 0f && look.barColor != 0u) {
                            val inset = 2f
                            drawRect(
                                Color(look.barColor.toInt()).copy(alpha = 0.75f),
                                Offset(xOf(c) + inset, y + inset * 1.5f),
                                Size(((cw - 2 * inset) * look.bar).coerceAtLeast(1f), rh - 3 * inset),
                            )
                        }
                        // A conditional format's icon at the cell's left, as Excel draws icon sets.
                        if (look.iconKind.toInt() != 0) drawCfIcon(look, Offset(xOf(c), y), rh)
                    }
                    // Excel's red corner on cells that have a note.
                    if (noteCells.isNotEmpty()) for (c in colList) {
                        if (r.toLong() * 16384 + c !in noteCells) continue
                        val cw = columnAxis.size(c) * k
                        val t = minOf(7.dp.toPx(), cw / 3, rh / 2)
                        val x1 = xOf(c) + cw
                        val tri = androidx.compose.ui.graphics.Path().apply { moveTo(x1 - t, y); lineTo(x1, y); lineTo(x1, y + t); close() }
                        drawPath(tri, Color(0xFFD32F2F))
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
                        val align = look?.align?.toInt() ?: 0
                        // Room to the right: the cell, and the empty cells after it for left-aligned text.
                        var room = cw
                        if (align == 1 || align == 0) {
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

        // Pictures and charts, floating over the cells at their anchors, as Excel places them.
        if (charts.isNotEmpty()) clipRect(rowHeader, colHeader, size.width, size.height) {
            for ((box, pic) in charts) {
                val x = xOf(box.col.toInt()) + box.dx * k
                val y = yOf(box.row.toInt()) + box.dy * k
                val w = box.width * k
                val h = box.height * k
                if (x > size.width || y > size.height || x + w < rowHeader || y + h < colHeader) continue
                if (pic != null) {
                    drawImage(
                        pic.asImageBitmap(),
                        dstOffset = androidx.compose.ui.unit.IntOffset(x.roundToInt(), y.roundToInt()),
                        dstSize = androidx.compose.ui.unit.IntSize(w.roundToInt().coerceAtLeast(1), h.roundToInt().coerceAtLeast(1)),
                    )
                } else {
                    drawRect(Color(0xFFF4F4F4), Offset(x, y), Size(w, h))
                }
                drawRect(Color(0xFFBDBDBD), Offset(x, y), Size(w, h), style = Stroke(1f))
                if (box.isChart && box.index.toInt() == pickedChart) drawRect(colors.primary, Offset(x, y), Size(w, h), style = Stroke(3.dp.toPx()))
            }
        }

        // Selection: the range tinted and framed, its first cell framed.
        var r0 = min(selectedRow, endRow)
        var r1 = max(selectedRow, endRow)
        var c0 = min(selectedCol, endCol)
        var c1 = max(selectedCol, endCol)
        // A selection takes in every merged range it touches, as Excel's does (not a whole row or column).
        var grown = !wholeLine
        while (grown) {
            grown = false
            for (m in merges) {
                if (m[0] <= r1 && m[2] >= r0 && m[1] <= c1 && m[3] >= c0) {
                    if (m[0] < r0 || m[2] > r1 || m[1] < c0 || m[3] > c1) {
                        r0 = min(r0, m[0]); r1 = max(r1, m[2]); c0 = min(c0, m[1]); c1 = max(c1, m[3])
                        grown = true
                    }
                }
            }
        }
        clipRect(rowHeader, colHeader, size.width, size.height) {
            val x0 = xOf(c0)
            val y0 = yOf(r0)
            val w = (columnAxis.start(c1 + 1) - columnAxis.start(c0)) * k
            val h = (rowAxis.start(r1 + 1) - rowAxis.start(r0)) * k
            if (r0 != r1 || c0 != c1) drawRect(colors.primary.copy(alpha = 0.12f), Offset(x0, y0), Size(w, h))
            drawRect(colors.primary, Offset(x0, y0), Size(w, h), style = Stroke(2.dp.toPx()))
        }

        // The cells a formula being typed refers to: each range framed and tinted in its own colour.
        if (formulaRefs.isNotEmpty()) clipRect(rowHeader, colHeader, size.width, size.height) {
            formulaRefs.forEachIndexed { i, ref ->
                val tint = REFERENCE_COLORS[i % REFERENCE_COLORS.size]
                val x0 = xOf(ref[1])
                val y0 = yOf(ref[0])
                val w = (columnAxis.start(ref[3] + 1) - columnAxis.start(ref[1])) * k
                val h = (rowAxis.start(ref[2] + 1) - rowAxis.start(ref[0])) * k
                drawRect(tint.copy(alpha = 0.10f), Offset(x0, y0), Size(w, h))
                drawRect(tint, Offset(x0, y0), Size(w, h), style = Stroke(2.dp.toPx()))
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
    // With TalkBack on: an invisible node over each visible cell ("B3: 1.250,00"), so a screen
    // reader moves through the sheet cell by cell; a double tap selects the cell (a second one types
    // in it, as a tap does). With TalkBack off none of this is composed.
    if (screenReader && viewSize.width > 0 && viewSize.height > 0) {
        val k = unit * scale
        val frozenW = columnAxis.start(fc) * k
        val frozenH = rowAxis.start(fr) * k
        val bodyLeft = rowHeader + frozenW
        val bodyTop = colHeader + frozenH
        val viewW = viewSize.width.toFloat()
        val viewH = viewSize.height.toFloat()
        fun xOf(c: Int) = if (c < fc) rowHeader + columnAxis.start(c) * k else bodyLeft + (columnAxis.start(c) - columnAxis.start(fc)) * k - offsetX
        fun yOf(r: Int) = if (r < fr) colHeader + rowAxis.start(r) * k else bodyTop + (rowAxis.start(r) - rowAxis.start(fr)) * k - offsetY
        val firstCol = max(fc, columnAxis.at(columnAxis.start(fc) + offsetX / k))
        val firstRow = max(fr, rowAxis.at(rowAxis.start(fr) + offsetY / k))
        val lastCol = min(cols - 1, columnAxis.at(columnAxis.start(fc) + (offsetX + viewW - bodyLeft) / k))
        val lastRow = min(rows - 1, rowAxis.at(rowAxis.start(fr) + (offsetY + viewH - bodyTop) / k))
        val visibleRows = (0 until fr) + (firstRow..lastRow)
        val visibleCols = (0 until fc) + (firstCol..lastCol)
        val maxX = max(0f, (columnAxis.start(cols) - columnAxis.start(fc)) * k - (viewW - bodyLeft))
        val maxY = max(0f, (rowAxis.start(rows) - rowAxis.start(fr)) * k - (viewH - bodyTop))
        val empty = stringResource(R.string.a11y_cell_empty)
        val selectLabel = stringResource(R.string.a11y_select_cell)
        Box(
            Modifier.fillMaxSize().semantics {
                collectionInfo = CollectionInfo(rowCount = rows, columnCount = cols)
                // TalkBack's scroll gestures move the sheet, as a finger would.
                horizontalScrollAxisRange = ScrollAxisRange(value = { offsetX }, maxValue = { maxX })
                verticalScrollAxisRange = ScrollAxisRange(value = { offsetY }, maxValue = { maxY })
                scrollBy { dx, dy ->
                    offsetX = (offsetX + dx).coerceIn(0f, maxX)
                    offsetY = (offsetY + dy).coerceIn(0f, maxY)
                    true
                }
            },
        ) {
            var placed = 0
            for (r in visibleRows) {
                if (rowAxis.size(r) <= 0f) continue // a hidden row
                for (c in visibleCols) {
                    if (placed >= MAX_SPOKEN_CELLS) break
                    if (columnAxis.size(c) <= 0f) continue
                    // A merged range is one node, at its first cell (where its value lives).
                    val m = mergeAt(r, c)
                    if (m != null && (m[0] != r || m[1] != c)) continue
                    val lastR = m?.get(2) ?: r
                    val lastC = m?.get(3) ?: c
                    // The part of the cell on screen, outside the headers and the frozen panes it scrolls under.
                    val x0 = max(xOf(c), if (c < fc) rowHeader else bodyLeft)
                    val y0 = max(yOf(r), if (r < fr) colHeader else bodyTop)
                    val x1 = min(xOf(c) + (columnAxis.start(lastC + 1) - columnAxis.start(c)) * k, if (c < fc) bodyLeft else viewW)
                    val y1 = min(yOf(r) + (rowAxis.start(lastR + 1) - rowAxis.start(r)) * k, if (r < fr) bodyTop else viewH)
                    if (x1 - x0 < 1f || y1 - y0 < 1f) continue
                    placed++
                    val name = columnName(c) + (r + 1)
                    val label = "$name: " + (cellText(sheet, r, c)?.takeIf { it.isNotBlank() } ?: empty)
                    val isSelected = r == selectedRow && c == selectedCol
                    key(r, c) {
                        Box(
                            Modifier
                                .offset { IntOffset(x0.roundToInt(), y0.roundToInt()) }
                                .size(with(density) { (x1 - x0).toDp() }, with(density) { (y1 - y0).toDp() })
                                .semantics {
                                    contentDescription = label
                                    if (isSelected) this.selected = true
                                    collectionItemInfo = CollectionItemInfo(rowIndex = r, rowSpan = lastR - r + 1, columnIndex = c, columnSpan = lastC - c + 1)
                                    onClick(label = selectLabel) {
                                        val now = selNow
                                        if (r == now[0] && c == now[1] && now[2] == now[0] && now[3] == now[1]) edit() else select(r, c)
                                        true
                                    }
                                },
                        )
                    }
                }
            }
        }
    }
    // Typing in the cell: a field laid exactly over it (at least a comfortable size).
    if (cellEditor != null) {
        val k = unit * scale
        val frozenW = columnAxis.start(fc) * k
        val frozenH = rowAxis.start(fr) * k
        val x = if (selectedCol < fc) rowHeader + columnAxis.start(selectedCol) * k else rowHeader + frozenW + (columnAxis.start(selectedCol) - columnAxis.start(fc)) * k - offsetX
        val y = if (selectedRow < fr) colHeader + rowAxis.start(selectedRow) * k else colHeader + frozenH + (rowAxis.start(selectedRow) - rowAxis.start(fr)) * k - offsetY
        // A merged cell is typed in over its whole area.
        val m = merges.firstOrNull { it[0] == selectedRow && it[1] == selectedCol }
        val lastCol = m?.get(3) ?: selectedCol
        val lastRow = m?.get(2) ?: selectedRow
        val w = max((columnAxis.start(lastCol + 1) - columnAxis.start(selectedCol)) * k, with(density) { 120.dp.toPx() })
        val h = max((rowAxis.start(lastRow + 1) - rowAxis.start(selectedRow)) * k, with(density) { 40.dp.toPx() })
        cellEditor(
            Modifier
                .offset { IntOffset(max(x, rowHeader).roundToInt(), max(y, colHeader).roundToInt()) }
                .size(with(density) { w.toDp() }, with(density) { h.toDp() }),
        )
    }
    }
}

/** Red, amber, green: low, middle and high icons (more icons blend between red and green). */
private fun iconColor(index: Int, count: Int): Color {
    if (count <= 3) return listOf(Color(0xFFD13B2F), Color(0xFFF2B82E), Color(0xFF2E9E4F)).getOrElse(index) { Color.Gray }
    val t = index.toFloat() / (count - 1).coerceAtLeast(1)
    return Color(red = (0xD1 * (1 - t) + 0x2E * t) / 255f, green = (0x3B * (1 - t) + 0x9E * t) / 255f, blue = (0x2F * (1 - t) + 0x4F * t) / 255f)
}

/** A conditional format's icon in a cell whose top-left corner is [at] and height [rowH]. */
private fun androidx.compose.ui.graphics.drawscope.DrawScope.drawCfIcon(look: CellLook, at: Offset, rowH: Float) {
    val side = minOf(rowH * 0.62f, 16.dp.toPx())
    val c = Offset(at.x + 3.dp.toPx() + side / 2, at.y + rowH / 2)
    val s = side / 2
    val i = look.iconIndex.toInt()
    val n = look.iconCount.toInt().coerceAtLeast(1)
    val col = iconColor(i, n)
    when (look.iconKind.toInt()) {
        1 -> {
            // Up for the highest, down for the lowest, sideways between.
            val path = androidx.compose.ui.graphics.Path().apply {
                when {
                    i + 1 == n -> { moveTo(c.x, c.y - s); lineTo(c.x + s, c.y + 0.1f * s); lineTo(c.x - s, c.y + 0.1f * s) }
                    i == 0 -> { moveTo(c.x, c.y + s); lineTo(c.x - s, c.y - 0.1f * s); lineTo(c.x + s, c.y - 0.1f * s) }
                    else -> { moveTo(c.x + s, c.y); lineTo(c.x - 0.1f * s, c.y - s); lineTo(c.x - 0.1f * s, c.y + s) }
                }
                close()
            }
            drawPath(path, col)
        }
        2 -> {
            val share = i.toFloat() / (n - 1).coerceAtLeast(1)
            drawCircle(Color.Gray, s - 0.5f, c, style = Stroke(1f))
            val h = (side - 4f) * share
            drawRect(Color(0xFF2B7CD3), Offset(c.x - s + 2f, c.y + s - 2f - h), Size(side - 4f, h))
        }
        else -> drawCircle(col, s - 0.5f, c)
    }
}

/** Excel's colours for the references in a formula being typed, in turn. */
private val REFERENCE_COLORS = listOf(
    Color(0xFF4472C4), Color(0xFFC0504D), Color(0xFF8064A2), Color(0xFF4BACC6), Color(0xFF9BBB59), Color(0xFFF79646),
)

// Consume a qualified range as one token, so its final cell cannot become a local reference.
private val REFERENCE = Regex("""(?<![\p{L}\p{N}_.!'"\]])((?:'(?:[^']|'')+'|(?:\[[^\]]+\])?[\p{L}\p{N}_.]+)!)?\$?([A-Za-z]{1,3})\$?([0-9]{1,7})(?::\$?([A-Za-z]{1,3})\$?([0-9]{1,7}))?(?![\p{L}\p{N}_(!.])""")

/**
 * The cell ranges a formula refers to on its own sheet ("=TOPLA(A1:B3;C5)" → A1:B3, C5), as
 * [first row, first column, last row, last column]; text in quotes and other sheets are skipped.
 */
fun formulaReferences(formula: String): List<IntArray> {
    if (!formula.startsWith("=")) return emptyList()
    // Quoted text is not a reference: blank it out, keeping positions.
    val plain = Regex(""""[^"]*"?""").replace(formula) { " ".repeat(it.value.length) }
    fun col(letters: String): Int = letters.uppercase(java.util.Locale.ROOT).fold(0) { n, ch -> n * 26 + (ch - 'A' + 1) } - 1
    return REFERENCE.findAll(plain).mapNotNull { m ->
        val (sheet, c1, r1, c2, r2) = m.destructured
        if (sheet.isNotEmpty()) return@mapNotNull null
        val a = intArrayOf(r1.toInt() - 1, col(c1))
        val b = if (c2.isEmpty()) a else intArrayOf(r2.toInt() - 1, col(c2))
        if (a[0] !in 0 until MAX_ROWS || b[0] !in 0 until MAX_ROWS || a[1] !in 0 until MAX_COLS || b[1] !in 0 until MAX_COLS) null
        else intArrayOf(minOf(a[0], b[0]), minOf(a[1], b[1]), maxOf(a[0], b[0]), maxOf(a[1], b[1]))
    }.distinctBy { it.toList() }.take(12).toList()
}
