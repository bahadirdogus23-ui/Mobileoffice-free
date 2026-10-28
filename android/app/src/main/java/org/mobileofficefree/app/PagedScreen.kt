package org.mobileofficefree.app

import android.content.Intent
import androidx.activity.compose.LocalActivity
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.calculateCentroid
import androidx.compose.foundation.gestures.calculateZoom
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.scrollBy
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.BottomAppBar
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.ColorMatrix
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlin.math.roundToInt
import kotlinx.coroutines.FlowPreview
import kotlinx.coroutines.flow.debounce
import kotlinx.coroutines.launch

private const val MAX_ZOOM = 5f
private const val DOUBLE_TAP_ZOOM = 2.5f

/** The widest page picture drawn; zooming further scales it up (sharp tiles come with the zoom work in phase 1). */
private const val MAX_RENDER_WIDTH = 2400

/** Night mode for pages whose engine cannot draw dark ones: invert every colour on screen. */
private val invertColors = ColorFilter.colorMatrix(
    ColorMatrix(
        floatArrayOf(
            -1f, 0f, 0f, 0f, 255f,
            0f, -1f, 0f, 0f, 255f,
            0f, 0f, -1f, 0f, 255f,
            0f, 0f, 0f, 1f, 0f,
        ),
    ),
)

/**
 * Word, PDF and presentations: pages one under another with a page counter. Pinch or double tap
 * to zoom, tap the counter to go to a page; the bottom bar has reading options, print and share.
 */
@OptIn(ExperimentalMaterial3Api::class, FlowPreview::class)
@Composable
fun PagedScreen(vm: PagedViewModel, state: PagedUiState, reading: ReadingSettings) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val list = rememberLazyListState()
    val hScroll = rememberScrollState()
    val current by remember { derivedStateOf { list.firstVisibleItemIndex + 1 } }
    var zoom by remember(state.uri) { mutableFloatStateOf(1f) }
    // The zoom a gesture has added so far; it is folded into [zoom] when the fingers lift.
    var pinch by remember { mutableFloatStateOf(1f) }
    var focus by remember { mutableStateOf(Offset.Zero) }
    val lastZoom = remember(state.uri) { floatArrayOf(1f) }
    var showReading by remember { mutableStateOf(false) }
    var showGoTo by remember { mutableStateOf(false) }
    val night = reading.night
    val invertOnScreen = night && !vm.drawsNight

    LaunchedEffect(state.uri) {
        snapshotFlow { list.firstVisibleItemIndex }.debounce(500).collect(vm::reportPosition)
    }
    // After a zoom, scroll so the point under the fingers stays under them.
    LaunchedEffect(zoom) {
        val ratio = zoom / lastZoom[0]
        lastZoom[0] = zoom
        if (ratio == 1f) return@LaunchedEffect
        withFrameNanos { }
        hScroll.scrollTo(((hScroll.value + focus.x) * ratio - focus.x).roundToInt())
        list.scrollToItem(list.firstVisibleItemIndex, (list.firstVisibleItemScrollOffset * ratio).roundToInt())
        list.scrollBy(focus.y * (ratio - 1))
    }

    val view = LocalView.current
    DisposableEffect(reading.keepScreenOn) {
        view.keepScreenOn = reading.keepScreenOn
        onDispose { view.keepScreenOn = false }
    }
    val activity = LocalActivity.current as? MainActivity
    DisposableEffect(activity, reading.volumeKeys, state.uri) {
        if (reading.volumeKeys) {
            activity?.volumeKeyHandler = { step ->
                val first = list.firstVisibleItemIndex
                // Back from the middle of a page goes to its top first.
                val target = if (step < 0 && list.firstVisibleItemScrollOffset > 0) first else first + step
                scope.launch { list.animateScrollToItem(target.coerceIn(0, state.pages.lastIndex)) }
            }
        }
        onDispose { activity?.volumeKeyHandler = null }
    }

    Column(Modifier.fillMaxSize()) {
        TopAppBar(
            title = { Text(state.fileName.substringBeforeLast('.'), maxLines = 1) },
            navigationIcon = {
                IconButton(onClick = vm::close) {
                    Icon(painterResource(R.drawable.ic_arrow_back), contentDescription = stringResource(R.string.back))
                }
            },
            actions = {
                TextButton(onClick = { showGoTo = true }) {
                    Text(
                        stringResource(R.string.page_counter, current, state.pages.size),
                        style = MaterialTheme.typography.titleMedium,
                    )
                }
            },
        )
        BoxWithConstraints(
            Modifier
                .weight(1f)
                .fillMaxWidth()
                .clipToBounds()
                .background(if (night) Color(0xFF121212) else Color(0xFF3A3A3A))
                .pointerInput(state.uri) {
                    // Two fingers zoom; one finger is left to the list for scrolling.
                    awaitEachGesture {
                        awaitFirstDown(requireUnconsumed = false)
                        var pinched = false
                        do {
                            val event = awaitPointerEvent(PointerEventPass.Initial)
                            if (event.changes.count { it.pressed } >= 2) {
                                pinched = true
                                focus = event.calculateCentroid(useCurrent = true)
                                pinch = (pinch * event.calculateZoom()).coerceIn(1f / zoom, MAX_ZOOM / zoom)
                                event.changes.forEach { it.consume() }
                            }
                        } while (event.changes.any { it.pressed })
                        if (pinched) {
                            zoom = (zoom * pinch).coerceIn(1f, MAX_ZOOM)
                            pinch = 1f
                        }
                    }
                }
                .pointerInput(state.uri) {
                    detectTapGestures(onDoubleTap = { at ->
                        focus = at
                        zoom = if (zoom > 1.01f) 1f else DOUBLE_TAP_ZOOM
                    })
                },
        ) {
            val widthPx = constraints.maxWidth
            val renderPx = (widthPx * zoom).roundToInt().coerceAtMost(maxOf(widthPx, MAX_RENDER_WIDTH))
            val heightPx = constraints.maxHeight.toFloat()
            val contentWidth = maxWidth * zoom
            Box(
                Modifier.fillMaxSize().graphicsLayer {
                    scaleX = pinch
                    scaleY = pinch
                    transformOrigin = TransformOrigin(focus.x / widthPx.coerceAtLeast(1), focus.y / heightPx.coerceAtLeast(1f))
                },
            ) {
                LazyColumn(
                    state = list,
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                    modifier = Modifier.fillMaxHeight().horizontalScroll(hScroll).width(contentWidth),
                ) {
                    items(state.pages.size, key = { it }) { i ->
                        val (w, h) = state.pages[i]
                        val paper = when {
                            !night -> Color.White
                            invertOnScreen -> Color.Black
                            else -> Color(0xFF333333)
                        }
                        Box(Modifier.fillMaxWidth().aspectRatio(w / h).background(paper)) {
                            val bmp = vm.page(i, renderPx, night)
                            if (bmp != null) {
                                Image(
                                    bmp.asImageBitmap(),
                                    contentDescription = stringResource(R.string.page_counter, i + 1, state.pages.size),
                                    contentScale = ContentScale.FillWidth,
                                    colorFilter = if (invertOnScreen) invertColors else null,
                                    modifier = Modifier.fillMaxSize(),
                                )
                            }
                        }
                    }
                }
            }
        }
        BottomAppBar {
            TextButton(onClick = { showReading = true }) { Text(stringResource(R.string.reading)) }
            TextButton(onClick = { showGoTo = true }) { Text(stringResource(R.string.go_to_page)) }
            TextButton(onClick = { DocumentPrinter.print(context, vm, state) }) { Text(stringResource(R.string.print)) }
            TextButton(onClick = {
                val send = Intent(Intent.ACTION_SEND)
                    .setType(context.contentResolver.getType(state.uri) ?: "*/*")
                    .putExtra(Intent.EXTRA_STREAM, state.uri)
                    .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                context.startActivity(Intent.createChooser(send, state.fileName))
            }) { Text(stringResource(R.string.share)) }
        }
    }

    if (showReading) {
        ModalBottomSheet(onDismissRequest = { showReading = false }) {
            Text(
                stringResource(R.string.reading),
                style = MaterialTheme.typography.titleLarge,
                modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
            )
            SettingRow(stringResource(R.string.night_mode), stringResource(R.string.night_mode_hint), reading.night, reading::setNight)
            SettingRow(stringResource(R.string.keep_screen_on), null, reading.keepScreenOn, reading::setKeepScreenOn)
            SettingRow(stringResource(R.string.volume_keys), stringResource(R.string.volume_keys_hint), reading.volumeKeys, reading::setVolumeKeys)
            Spacer(Modifier.height(24.dp))
        }
    }

    if (showGoTo) {
        var text by remember { mutableStateOf("") }
        val target = text.toIntOrNull()?.takeIf { it in 1..state.pages.size }
        AlertDialog(
            onDismissRequest = { showGoTo = false },
            title = { Text(stringResource(R.string.go_to_page)) },
            text = {
                OutlinedTextField(
                    value = text,
                    onValueChange = { text = it.filter(Char::isDigit).take(6) },
                    label = { Text(stringResource(R.string.page_number_range, state.pages.size)) },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                )
            },
            confirmButton = {
                TextButton(enabled = target != null, onClick = {
                    showGoTo = false
                    if (target != null) scope.launch { list.scrollToItem(target - 1) }
                }) { Text(stringResource(R.string.go)) }
            },
            dismissButton = { TextButton(onClick = { showGoTo = false }) { Text(stringResource(R.string.cancel)) } },
        )
    }
}

@Composable
private fun SettingRow(title: String, hint: String?, checked: Boolean, onChange: (Boolean) -> Unit) {
    ListItem(
        headlineContent = { Text(title) },
        supportingContent = if (hint != null) { { Text(hint) } } else null,
        trailingContent = { Switch(checked = checked, onCheckedChange = onChange) },
        modifier = Modifier.clickable { onChange(!checked) },
    )
}
