package org.mobileofficefree.app

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectDragGesturesAfterLongPress
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import androidx.compose.ui.window.DialogWindowProvider
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import java.util.Locale
import kotlin.math.abs
import kotlinx.coroutines.delay

/**
 * A presentation shown full screen, one slide at a time: tap the right side or swipe left for the
 * next slide, the left side or swipe right for the previous one; touch and hold to point with a
 * red dot. A bar at the bottom (tap the middle to show or hide it) has the slide number, a timer
 * and the speaker notes.
 */
@Composable
fun PresentScreen(vm: SlidesViewModel, state: PagedUiState, start: Int, onClose: () -> Unit) {
    var slide by remember { mutableIntStateOf(start.coerceIn(0, state.pages.lastIndex.coerceAtLeast(0))) }
    var bar by remember { mutableStateOf(false) }
    var showNotes by remember { mutableStateOf(false) }
    var pointer by remember { mutableStateOf<Offset?>(null) }
    val started = remember { System.currentTimeMillis() }
    var now by remember { mutableLongStateOf(started) }
    LaunchedEffect(Unit) {
        while (true) {
            now = System.currentTimeMillis()
            delay(1_000)
        }
    }
    val notes by produceState("", slide) { value = vm.notes(slide) }
    val last = state.pages.lastIndex
    val go = { step: Int -> slide = (slide + step).coerceIn(0, last) }

    Dialog(
        onDismissRequest = onClose,
        properties = DialogProperties(usePlatformDefaultWidth = false, decorFitsSystemWindows = false),
    ) {
        // Full screen: hide the status and navigation bars; keep the screen on while presenting.
        val window = (LocalView.current.parent as? DialogWindowProvider)?.window
        val view = LocalView.current
        DisposableEffect(window) {
            view.keepScreenOn = true
            window?.let {
                WindowCompat.setDecorFitsSystemWindows(it, false)
                WindowInsetsControllerCompat(it, view).apply {
                    hide(WindowInsetsCompat.Type.systemBars())
                    systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
                }
            }
            onDispose { view.keepScreenOn = false }
        }
        BackHandler(onBack = onClose)
        BoxWithConstraints(Modifier.fillMaxSize().background(Color.Black), contentAlignment = Alignment.Center) {
            val widthPx = constraints.maxWidth
            val (w, h) = state.pages.getOrElse(slide) { 16f to 9f }
            Box(
                Modifier
                    .fillMaxWidth()
                    .aspectRatio(w / h, matchHeightConstraintsFirst = maxWidth / maxHeight > w / h)
                    .pointerInput(slide) {
                        detectTapGestures { at ->
                            when {
                                at.x > size.width * 0.66f -> go(1)
                                at.x < size.width * 0.33f -> go(-1)
                                else -> bar = !bar
                            }
                        }
                    }
                    .pointerInput(slide) {
                        var total = 0f
                        detectHorizontalDragGestures(
                            onDragStart = { total = 0f },
                            onDragEnd = { if (abs(total) > 80f) go(if (total < 0) 1 else -1) },
                        ) { _, dx -> total += dx }
                    }
                    .pointerInput(slide) {
                        detectDragGesturesAfterLongPress(
                            onDragStart = { pointer = it },
                            onDragEnd = { pointer = null },
                            onDragCancel = { pointer = null },
                        ) { change, _ -> pointer = change.position }
                    },
            ) {
                vm.page(slide, widthPx, false)?.let {
                    Image(it.asImageBitmap(), contentDescription = stringResource(R.string.page_counter, slide + 1, last + 1), contentScale = ContentScale.Fit, modifier = Modifier.fillMaxSize())
                }
                pointer?.let { p ->
                    Canvas(Modifier.fillMaxSize()) {
                        drawCircle(Color(0x66FF1744), radius = 22.dp.toPx(), center = p)
                        drawCircle(Color(0xFFFF1744), radius = 8.dp.toPx(), center = p)
                    }
                }
            }
            // Draw the next slide ahead of time so that moving on is instant.
            if (slide < last) vm.page(slide + 1, widthPx, false)
            if (bar) {
                Column(Modifier.align(Alignment.BottomCenter).fillMaxWidth().background(Color(0xCC000000)).padding(8.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        TextButton(onClick = { go(-1) }, enabled = slide > 0) { Text("◀", color = Color.White) }
                        Text(stringResource(R.string.page_counter, slide + 1, last + 1), color = Color.White)
                        TextButton(onClick = { go(1) }, enabled = slide < last) { Text("▶", color = Color.White) }
                        val secs = (now - started) / 1000
                        Text("%d:%02d".format(Locale.ROOT, secs / 60, secs % 60), color = Color.White, modifier = Modifier.padding(horizontal = 12.dp))
                        Box(Modifier.weight(1f))
                        TextButton(onClick = { showNotes = !showNotes }) { Text(stringResource(R.string.speaker_notes), color = Color.White) }
                        TextButton(onClick = onClose) { Text(stringResource(R.string.end_show), color = Color.White) }
                    }
                    if (showNotes) {
                        Text(
                            notes.ifBlank { stringResource(R.string.no_notes) },
                            color = Color.White,
                            style = MaterialTheme.typography.bodyLarge,
                            modifier = Modifier.heightIn(max = 160.dp).verticalScroll(rememberScrollState()).padding(8.dp),
                        )
                    }
                }
            }
        }
    }
}
