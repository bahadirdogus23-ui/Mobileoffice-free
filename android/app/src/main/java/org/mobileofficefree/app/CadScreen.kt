package org.mobileofficefree.app

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.compose.ui.platform.LocalContext
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Checkbox
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.PrimaryScrollableTabRow
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.drawscope.withTransform
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.FlowPreview
import kotlinx.coroutines.flow.debounce

/** DWG/DXF viewer: pinch to zoom, drag to pan, double tap to fit; layers and background in the top bar. */
@OptIn(ExperimentalMaterial3Api::class, FlowPreview::class)
@Composable
fun CadScreen(vm: CadViewModel, state: CadUiState) {
    var showLayers by remember { mutableStateOf(false) }
    BackHandler { vm.close() }
    // A sharp picture once the fingers rest; the last one is moved and scaled meanwhile.
    LaunchedEffect(Unit) {
        snapshotFlow { vm.view }.debounce(150).collect { vm.render() }
    }

    Column(Modifier.fillMaxSize().navigationBarsPadding()) {
        TopAppBar(
            title = { Text(state.fileName.substringBeforeLast('.'), maxLines = 1) },
            navigationIcon = {
                IconButton(onClick = vm::close) {
                    Icon(painterResource(R.drawable.ic_arrow_back), contentDescription = stringResource(R.string.back))
                }
            },
        )
        Box(
            Modifier
                .weight(1f)
                .fillMaxWidth()
                .background(if (state.dark) Color(0xFF212830) else Color.White)
                .onSizeChanged { vm.setViewport(it.width, it.height) }
                .pointerInput(state.uri) {
                    detectTransformGestures { centroid, pan, zoom, _ -> vm.transform(centroid.x, centroid.y, pan.x, pan.y, zoom) }
                }
                .pointerInput(state.uri, vm.measuring) {
                    if (vm.measuring) {
                        // A tap measures where it lands (snapped); holding and sliding aims a point
                        // above the finger, its snap shown live, and lifting measures it.
                        val lift = 64.dp.toPx()
                        awaitEachGesture {
                            val down = awaitFirstDown()
                            var aiming = false
                            while (true) {
                                val event = awaitPointerEvent()
                                if (event.changes.size > 1) {
                                    // A second finger: zooming, not measuring.
                                    vm.aimCancel()
                                    return@awaitEachGesture
                                }
                                val c = event.changes.firstOrNull { it.id == down.id } ?: break
                                if (!c.pressed) {
                                    if (aiming) vm.aimDone() else vm.measureTap(c.position.x, c.position.y)
                                    break
                                }
                                if (!aiming && (c.position - down.position).getDistance() > viewConfiguration.touchSlop) aiming = true
                                if (aiming) {
                                    c.consume()
                                    vm.aimAt(c.position.x, c.position.y - lift)
                                }
                            }
                        }
                    } else {
                        detectTapGestures(onDoubleTap = { vm.fit() })
                    }
                },
        ) {
            val frame = vm.frame
            val view = vm.view
            Canvas(Modifier.fillMaxSize()) {
                if (frame == null || view == null) return@Canvas
                // Map the drawn frame onto the current view: scale k about the centre, then shift.
                val k = (view.scale / frame.view.scale).toFloat()
                val tx = ((frame.view.centerX - view.centerX) * view.scale).toFloat()
                val ty = (-(frame.view.centerY - view.centerY) * view.scale).toFloat()
                val half = Offset(size.width / 2, size.height / 2)
                withTransform({
                    translate(half.x + tx, half.y + ty)
                    scale(k, k, pivot = Offset.Zero)
                    translate(-half.x, -half.y)
                }) {
                    drawImage(frame.bitmap.asImageBitmap())
                }
            }
            if (vm.measuring) MeasureOverlay(vm, state.dark)
        }
        if (state.spaces.size > 1) {
            PrimaryScrollableTabRow(selectedTabIndex = state.space) {
                state.spaces.forEachIndexed { i, name ->
                    Tab(selected = i == state.space, onClick = { vm.selectSpace(i) }, text = { Text(name) })
                }
            }
        }
        val savePdf = rememberLauncherForActivityResult(CreateFile()) { uri -> if (uri != null) vm.exportPdf(uri) }
        var askPdfArea by remember { mutableStateOf(false) }
        if (askPdfArea) {
            androidx.compose.material3.AlertDialog(
                onDismissRequest = { askPdfArea = false },
                title = { Text(stringResource(R.string.cad_pdf_area)) },
                text = {
                    androidx.compose.foundation.layout.Column {
                        for ((onlyView, label) in listOf(false to R.string.cad_pdf_whole, true to R.string.cad_pdf_view)) {
                            androidx.compose.material3.ListItem(
                                headlineContent = { Text(stringResource(label)) },
                                modifier = Modifier.clickable {
                                    askPdfArea = false
                                    vm.pdfOnlyView = onlyView
                                    savePdf.launch(CreateFile.Request(state.fileName.substringBeforeLast('.') + ".pdf", "application/pdf"))
                                },
                            )
                        }
                    }
                },
                confirmButton = { androidx.compose.material3.TextButton(onClick = { askPdfArea = false }) { Text(stringResource(R.string.cancel)) } },
            )
        }
        val context = LocalContext.current
        SoftToolbar {
            ToolItem("⤢", stringResource(R.string.cad_fit), onClick = vm::fit)
            ToolItem(
                "📏",
                stringResource(if (vm.measuring) R.string.cad_measure_stop else R.string.cad_measure),
                onClick = { if (vm.measuring) vm.stopMeasuring() else vm.measuring = true },
            )
            ToolItem("☰", stringResource(R.string.cad_layers), onClick = { showLayers = true })
            ToolItem(if (state.dark) "☀" else "☾", stringResource(if (state.dark) R.string.cad_white_sheet else R.string.cad_dark_sheet), onClick = vm::toggleBackground)
            ToolItem("⇩", stringResource(R.string.cad_save_pdf)) { askPdfArea = true }
            ToolItem("⇪", stringResource(R.string.share)) {
                val send = android.content.Intent(android.content.Intent.ACTION_SEND)
                    .setType("application/octet-stream")
                    .putExtra(android.content.Intent.EXTRA_STREAM, state.uri)
                    .addFlags(android.content.Intent.FLAG_GRANT_READ_URI_PERMISSION)
                context.startActivity(android.content.Intent.createChooser(send, state.fileName))
            }
        }
    }

    if (showLayers) {
        ModalBottomSheet(onDismissRequest = { showLayers = false }) {
            Text(
                stringResource(R.string.cad_layers),
                style = MaterialTheme.typography.titleLarge,
                modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
            )
            LazyColumn {
                items(state.layers, key = { it.name }) { layer ->
                    Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp), verticalAlignment = Alignment.CenterVertically) {
                        Checkbox(checked = layer.visible, onCheckedChange = { vm.setLayerVisible(layer.name, it) })
                        Box(Modifier.size(14.dp).background(Color(0xFF000000 or layer.color.toLong()), CircleShape))
                        Spacer(Modifier.width(12.dp))
                        Text(layer.name)
                    }
                }
            }
            Spacer(Modifier.size(24.dp))
        }
    }
}

/** The measured points, the line between them and its length (in drawing units). */
@Composable
private fun MeasureOverlay(vm: CadViewModel, dark: Boolean) {
    val points = vm.measure
    val accent = if (dark) Color(0xFFFFC107) else Color(0xFFD32F2F)
    Canvas(Modifier.fillMaxSize()) {
        val screen = points.mapNotNull { vm.screenOf(it) }.map { Offset(it.first, it.second) }
        if (screen.size == 2) drawLine(accent, screen[0], screen[1], strokeWidth = 3.dp.toPx())
        vm.aim?.let { (pt, kind) ->
            vm.screenOf(pt)?.let { (x, y) ->
                // The aimed point: a crosshair, its snap marked as a square.
                val c = Offset(x, y)
                val r = 16.dp.toPx()
                drawLine(accent, Offset(c.x - r, c.y), Offset(c.x + r, c.y), 1.5.dp.toPx())
                drawLine(accent, Offset(c.x, c.y - r), Offset(c.x, c.y + r), 1.5.dp.toPx())
                if (kind.isNotEmpty()) {
                    val q = 8.dp.toPx()
                    drawRect(accent, topLeft = Offset(c.x - q, c.y - q), size = androidx.compose.ui.geometry.Size(2 * q, 2 * q), style = androidx.compose.ui.graphics.drawscope.Stroke(2.5.dp.toPx()))
                }
                if (screen.size == 1) drawLine(accent.copy(alpha = 0.6f), screen[0], c, strokeWidth = 2.dp.toPx())
            }
        }
        screen.forEachIndexed { i, p ->
            // A snapped point is a square, as CAD programs mark them; a free one a dot.
            if (vm.measureSnaps.getOrNull(i).isNullOrEmpty()) {
                drawCircle(accent, radius = 6.dp.toPx(), center = p)
                drawCircle(Color.White, radius = 2.5.dp.toPx(), center = p)
            } else {
                val r = 7.dp.toPx()
                drawRect(accent, topLeft = Offset(p.x - r, p.y - r), size = androidx.compose.ui.geometry.Size(2 * r, 2 * r), style = androidx.compose.ui.graphics.drawscope.Stroke(2.5.dp.toPx()))
            }
        }
    }
    val snapNames = mapOf(
        "Endpoint" to R.string.snap_end, "Midpoint" to R.string.snap_mid, "Center" to R.string.snap_center,
        "Intersection" to R.string.snap_intersection, "Quadrant" to R.string.snap_quadrant,
        "Node" to R.string.snap_node, "Insertion" to R.string.snap_insertion,
    )
    val lastSnap = (vm.aim?.second ?: vm.measureSnaps.lastOrNull())?.let { snapNames[it] }?.let { stringResource(it) }
    val text = when (points.size) {
        0 -> stringResource(R.string.cad_measure_first) + (lastSnap?.let { " · $it" } ?: "")
        1 -> stringResource(R.string.cad_measure_second) + (lastSnap?.let { " · $it" } ?: "")
        else -> {
            val dx = points[1].first - points[0].first
            val dy = points[1].second - points[0].second
            stringResource(R.string.cad_measure_result, measureNumber(kotlin.math.hypot(dx, dy)), measureNumber(kotlin.math.abs(dx)), measureNumber(kotlin.math.abs(dy)))
        }
    }
    Box(Modifier.fillMaxSize().padding(12.dp), contentAlignment = Alignment.TopCenter) {
        Text(
            text,
            style = MaterialTheme.typography.titleSmall,
            color = Color.White,
            modifier = Modifier.background(Color(0xCC000000), RoundedCornerShape(12.dp)).padding(horizontal = 14.dp, vertical = 8.dp),
        )
    }
}

/** A length with as many decimals as it needs (Turkish decimal comma where the phone uses it). */
internal fun measureNumber(v: Double): String {
    val digits = when {
        v >= 1000 -> 1
        v >= 10 -> 2
        else -> 3
    }
    return String.format(java.util.Locale.getDefault(), "%,.${digits}f", v)
}
