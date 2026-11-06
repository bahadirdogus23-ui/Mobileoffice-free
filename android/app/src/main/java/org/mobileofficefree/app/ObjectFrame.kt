package org.mobileofficefree.app

import android.graphics.Bitmap
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.awaitTouchSlopOrCancellation
import androidx.compose.foundation.gestures.drag
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import kotlin.math.abs
import kotlin.math.atan2
import kotlin.math.cos
import kotlin.math.roundToInt
import kotlin.math.sin

/**
 * What an object looks like on its own, for dragging it as design apps do: [snapshot] is the
 * object's pixels (transparent elsewhere), placed at ([left], [top])–([right], [bottom]) as
 * fractions of the page; [backdrop], when given, is the whole page without the object.
 */
class ObjectLook(
    val snapshot: Bitmap,
    val left: Float,
    val top: Float,
    val right: Float,
    val bottom: Float,
    val backdrop: Bitmap? = null,
)

/**
 * A picked object's frame over a page, the way design apps handle it: drag the object to move it,
 * a corner to resize it (keeping its shape), the round handle above it to turn it. While the
 * fingers are down the object itself follows them ([look]), drawn over what is behind it (its
 * [ObjectLook.backdrop], or [cover] painted over where it was). [onDone] gets the move as
 * fractions of the page, the scale, and the turn in clockwise degrees.
 *
 * The box is ([l], [t])–([r], [b]) in fractions of the page, turned [rotation] degrees about its
 * centre. Touches that start away from the object are left to the page (scrolling, taps).
 */
@Composable
fun ObjectFrame(
    key: Any,
    l: Float,
    t: Float,
    r: Float,
    b: Float,
    rotation: Float = 0f,
    look: ObjectLook? = null,
    cover: Color? = null,
    canRotate: Boolean = true,
    onDone: (dx: Float, dy: Float, scale: Float, turn: Float) -> Unit,
) {
    val accent = MaterialTheme.colorScheme.primary
    val density = LocalDensity.current
    val handleR = with(density) { 7.dp.toPx() }
    val reach = with(density) { 28.dp.toPx() }
    val rotateGap = with(density) { 34.dp.toPx() }
    val pad = with(density) { 4.dp.toPx() }

    // The change being made, kept until the object comes back changed (a new [key]).
    var drag by remember(key) { mutableStateOf(Offset.Zero) }
    var scale by remember(key) { mutableFloatStateOf(1f) }
    var turn by remember(key) { mutableFloatStateOf(0f) }
    var active by remember(key) { mutableStateOf(false) }
    var box by remember { mutableStateOf(IntSize.Zero) }
    val done by rememberUpdatedState(onDone)

    fun center() = Offset((l + r) / 2 * box.width, (t + b) / 2 * box.height)
    fun half() = Offset((r - l) / 2 * box.width, (b - t) / 2 * box.height)

    /** Point (`x`, `y`) of the box (relative to its centre, unturned) on the page now. */
    fun onPage(x: Float, y: Float): Offset {
        val a = Math.toRadians((rotation + turn).toDouble())
        val c = center() + drag
        val sx = x * scale
        val sy = y * scale
        return Offset(c.x + (sx * cos(a) - sy * sin(a)).toFloat(), c.y + (sx * sin(a) + sy * cos(a)).toFloat())
    }

    Canvas(
        Modifier
            .fillMaxSize()
            .onSizeChanged { box = it }
            .pointerInput(key, l, t, r, b, rotation, canRotate) {
                awaitEachGesture {
                    val down = awaitFirstDown(requireUnconsumed = false)
                    val h = half()
                    val p = down.position
                    val mode = when {
                        canRotate && (p - onPage(0f, -h.y - rotateGap / scale)).getDistance() < reach -> Mode.TURN
                        listOf(-1f to -1f, 1f to -1f, -1f to 1f, 1f to 1f).any { (sx, sy) -> (p - onPage(sx * h.x, sy * h.y)).getDistance() < reach } -> Mode.SIZE
                        else -> {
                            // Inside the (turned, scaled) box?
                            val a = Math.toRadians(-(rotation + turn).toDouble())
                            val d = p - (center() + drag)
                            val ux = (d.x * cos(a) - d.y * sin(a)).toFloat()
                            val uy = (d.x * sin(a) + d.y * cos(a)).toFloat()
                            if (abs(ux) <= h.x * scale + pad && abs(uy) <= h.y * scale + pad) Mode.MOVE else null
                        }
                    } ?: return@awaitEachGesture
                    val startDrag = drag
                    val startScale = scale
                    val startTurn = turn
                    val c0 = center() + drag
                    val startDist = (p - c0).getDistance().coerceAtLeast(1f)
                    val startAngle = angleOf(p - c0)
                    val slop = awaitTouchSlopOrCancellation(down.id) { change, _ -> change.consume() } ?: return@awaitEachGesture
                    active = true
                    fun follow(at: Offset) {
                        when (mode) {
                            Mode.MOVE -> drag = startDrag + (at - p)
                            Mode.SIZE -> scale = (startScale * (at - c0).getDistance() / startDist).coerceIn(0.1f, 10f)
                            Mode.TURN -> {
                                var a = startTurn + angleOf(at - c0) - startAngle
                                // Settles on upright and quarter turns, as design apps do.
                                val whole = rotation + a
                                val nearest = (whole / 90f).roundToInt() * 90f
                                if (abs(whole - nearest) < 4f) a = nearest - rotation
                                turn = a
                            }
                        }
                    }
                    follow(slop.position)
                    val finished = drag(slop.id) { change ->
                        follow(change.position)
                        change.consume()
                    }
                    if (finished && (drag != startDrag || scale != startScale || turn != startTurn)) {
                        done(drag.x / box.width, drag.y / box.height, scale, turn)
                    } else if (!finished) {
                        drag = startDrag
                        scale = startScale
                        turn = startTurn
                        active = drag != Offset.Zero || scale != 1f || turn != 0f
                    }
                }
            },
    ) {
        val c = center()
        val h = half()
        val changed = active && (drag != Offset.Zero || scale != 1f || turn != 0f)
        if (changed) {
            // The object follows the fingers over what is behind it.
            val backdrop = look?.backdrop
            if (backdrop != null) {
                drawImage(backdrop.asImageBitmap(), dstSize = IntSize(size.width.toInt(), size.height.toInt()))
            } else if (cover != null) {
                rotate(rotation, pivot = c) { drawRect(cover, topLeft = c - h, size = Size(h.x * 2, h.y * 2)) }
            }
            look?.let { lk ->
                translate(drag.x, drag.y) {
                    rotate(turn, pivot = c) {
                        scale(scale, pivot = c) {
                            drawImage(
                                lk.snapshot.asImageBitmap(),
                                dstOffset = IntOffset((lk.left * size.width).roundToInt(), (lk.top * size.height).roundToInt()),
                                dstSize = IntSize(((lk.right - lk.left) * size.width).roundToInt().coerceAtLeast(1), ((lk.bottom - lk.top) * size.height).roundToInt().coerceAtLeast(1)),
                            )
                        }
                    }
                }
            }
        }
        // The frame, its corner handles and the turning handle.
        val corners = listOf(onPage(-h.x, -h.y), onPage(h.x, -h.y), onPage(h.x, h.y), onPage(-h.x, h.y))
        for (i in corners.indices) drawLine(accent, corners[i], corners[(i + 1) % 4], strokeWidth = 2.dp.toPx())
        for (p in corners) {
            drawCircle(Color.White, handleR + 1.5.dp.toPx(), p)
            drawCircle(accent, handleR, p)
        }
        if (canRotate) {
            val top = onPage(0f, -h.y)
            val knob = onPage(0f, -h.y - rotateGap / scale)
            drawLine(accent, top, knob, strokeWidth = 2.dp.toPx())
            drawCircle(Color.White, handleR + 3.dp.toPx(), knob)
            drawCircle(accent, handleR + 3.dp.toPx(), knob, style = Stroke(2.dp.toPx()))
            drawCircle(accent, 2.5.dp.toPx(), knob)
        }
    }
}

private enum class Mode { MOVE, SIZE, TURN }

/** Clockwise degrees of [v] from the x axis (screen y points down). */
private fun angleOf(v: Offset): Float = Math.toDegrees(atan2(v.y.toDouble(), v.x.toDouble())).toFloat()

/**
 * The pixels of [full] that differ from [without] (the same page drawn without one object): that
 * object on its own, cut to where it is, with where that is as fractions of the page. Null when
 * nothing differs.
 */
fun objectSnapshot(full: Bitmap, without: Bitmap): ObjectLook? {
    if (full.width != without.width || full.height != without.height) return null
    val w = full.width
    val h = full.height
    val a = IntArray(w * h).also { full.getPixels(it, 0, w, 0, 0, w, h) }
    val bg = IntArray(w * h).also { without.getPixels(it, 0, w, 0, 0, w, h) }
    var x0 = w
    var y0 = h
    var x1 = -1
    var y1 = -1
    for (y in 0 until h) {
        val row = y * w
        for (x in 0 until w) {
            if (differs(a[row + x], bg[row + x])) {
                if (x < x0) x0 = x
                if (x > x1) x1 = x
                if (y < y0) y0 = y
                if (y > y1) y1 = y
            }
        }
    }
    if (x1 < 0) return null
    val cw = x1 - x0 + 1
    val ch = y1 - y0 + 1
    val out = IntArray(cw * ch)
    for (y in 0 until ch) {
        for (x in 0 until cw) {
            val i = (y0 + y) * w + x0 + x
            out[y * cw + x] = if (differs(a[i], bg[i])) a[i] else 0
        }
    }
    val snap = Bitmap.createBitmap(out, cw, ch, Bitmap.Config.ARGB_8888)
    return ObjectLook(snap, x0.toFloat() / w, y0.toFloat() / h, (x1 + 1).toFloat() / w, (y1 + 1).toFloat() / h, without)
}

/** Whether two ARGB pixels differ by more than rounding. */
private fun differs(p: Int, q: Int): Boolean {
    if (p == q) return false
    val dr = abs(((p shr 16) and 0xFF) - ((q shr 16) and 0xFF))
    val dg = abs(((p shr 8) and 0xFF) - ((q shr 8) and 0xFF))
    val db = abs((p and 0xFF) - (q and 0xFF))
    return dr + dg + db > 12
}

/** A piece of [page] (fractions [l], [t], [r], [b]) as the look of an object on a plain page. */
fun croppedLook(page: Bitmap, l: Float, t: Float, r: Float, b: Float): ObjectLook? {
    val x0 = (l * page.width).toInt().coerceIn(0, page.width - 1)
    val y0 = (t * page.height).toInt().coerceIn(0, page.height - 1)
    val x1 = (r * page.width).toInt().coerceIn(x0 + 1, page.width)
    val y1 = (b * page.height).toInt().coerceIn(y0 + 1, page.height)
    val piece = Bitmap.createBitmap(page, x0, y0, x1 - x0, y1 - y0)
    return ObjectLook(piece, x0.toFloat() / page.width, y0.toFloat() / page.height, x1.toFloat() / page.width, y1.toFloat() / page.height)
}
