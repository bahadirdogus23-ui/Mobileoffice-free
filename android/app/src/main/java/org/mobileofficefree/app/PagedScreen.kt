package org.mobileofficefree.app

import android.content.Intent
import android.widget.Toast
import androidx.activity.compose.BackHandler
import androidx.activity.compose.LocalActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
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
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.rememberLazyGridState
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.BottomAppBar
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
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
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
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
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import kotlin.math.roundToInt
import kotlin.math.sqrt
import java.io.File
import kotlinx.coroutines.FlowPreview
import kotlinx.coroutines.flow.debounce
import kotlinx.coroutines.launch

private const val MAX_ZOOM = 5f
private val hitColor = Color(0x66FFD54F)
private val currentHitColor = Color(0x99FF9800)
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
    var showPages by remember { mutableStateOf(false) }
    // PDF page tools: pages picked in the Pages sheet, and where extracted pages / the edit go.
    val pdf = vm as? PdfViewModel
    var selecting by remember(state.uri) { mutableStateOf(false) }
    val selected = remember(state.uri) { mutableStateListOf<Int>() }
    var extractPages by remember { mutableStateOf<List<Int>>(emptyList()) }
    var closeAfterSave by remember { mutableStateOf(false) }
    var drawSignature by remember { mutableStateOf(false) }
    var showPassword by remember { mutableStateOf(false) }
    var showCompress by remember { mutableStateOf(false) }
    // A drawn signature waiting for a tap on the page it goes on.
    var placing by remember(state.uri) { mutableStateOf<Signature?>(null) }
    val extractFile = rememberLauncherForActivityResult(CreateFile()) { uri -> if (uri != null) pdf?.extractTo(uri, extractPages) }
    val saveEdited = rememberLauncherForActivityResult(CreateFile()) { uri ->
        if (uri != null) pdf?.saveTo(uri)
        if (uri != null && closeAfterSave) vm.answerClose(true)
        closeAfterSave = false
    }
    val addPdf = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri -> if (uri != null) pdf?.appendPdf(uri) }
    val editedName = convertedName(state.fileName, "pdf").removeSuffix(".pdf") + "-duzenlendi.pdf"
    val saveCompressed = rememberLauncherForActivityResult(CreateFile()) { uri -> pdf?.saveCompressed(uri) }
    LaunchedEffect(pdf?.compressed) {
        if (pdf?.compressed != null) saveCompressed.launch(CreateFile.Request(editedName.replace("-duzenlendi", "-kucuk"), "application/pdf"))
    }
    var showConvert by remember { mutableStateOf(false) }
    var converting by remember { mutableStateOf<ExportFormat?>(null) }
    val createFile = rememberLauncherForActivityResult(CreateFile()) { uri ->
        val format = converting
        if (uri != null && format != null) vm.exportTo(uri, format)
        converting = null
    }
    LaunchedEffect(vm.notice) {
        vm.notice?.let {
            Toast.makeText(context, it, Toast.LENGTH_LONG).show()
            vm.consumeNotice()
        }
    }
    var showSearch by remember(state.uri) { mutableStateOf(false) }
    val hitsByPage = remember(vm.found) { vm.found.groupBy { it.page } }
    val night = reading.night
    val invertOnScreen = night && !vm.drawsNight

    BackHandler { vm.requestClose() }
    // Follow the page being read aloud.
    LaunchedEffect(vm.readingPage) {
        if (vm.readingPage >= 0) list.animateScrollToItem(vm.readingPage)
    }
    // Leaving the document stops the voice.
    DisposableEffect(state.uri) { onDispose { vm.stopReading() } }
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
                IconButton(onClick = vm::requestClose) {
                    Icon(painterResource(R.drawable.ic_arrow_back), contentDescription = stringResource(R.string.back))
                }
            },
            actions = {
                if (pdf?.edited == true) {
                    TextButton(onClick = { saveEdited.launch(CreateFile.Request(editedName, "application/pdf")) }, enabled = !pdf.busy) {
                        Text(stringResource(R.string.save))
                    }
                }
                if (vm.searchable) {
                    TextButton(onClick = {
                        showSearch = !showSearch
                        if (!showSearch) vm.search("")
                    }) { Text(stringResource(R.string.search)) }
                }
                TextButton(onClick = { showGoTo = true }) {
                    Text(
                        stringResource(R.string.page_counter, current, state.pages.size),
                        style = MaterialTheme.typography.titleMedium,
                    )
                }
            },
        )
        if (pdf?.busy == true) LinearProgressIndicator(Modifier.fillMaxWidth())
        if (showSearch) {
            Row(Modifier.fillMaxWidth().padding(horizontal = 8.dp), verticalAlignment = Alignment.CenterVertically) {
                OutlinedTextField(
                    value = vm.query,
                    onValueChange = vm::search,
                    placeholder = { Text(stringResource(R.string.search_in_text)) },
                    singleLine = true,
                    modifier = Modifier.weight(1f),
                )
                Text(
                    when {
                        vm.query.isBlank() -> ""
                        vm.searching -> "…"
                        vm.matchCount == 0 -> stringResource(R.string.no_matches)
                        else -> stringResource(R.string.page_counter, vm.currentMatch + 1, vm.matchCount)
                    },
                    style = MaterialTheme.typography.labelMedium,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.widthIn(min = 56.dp).padding(horizontal = 4.dp),
                )
                TextButton(onClick = { vm.nextMatch(-1) }, enabled = vm.matchCount > 0) { Text("▲") }
                TextButton(onClick = { vm.nextMatch(1) }, enabled = vm.matchCount > 0) { Text("▼") }
            }
        }
        placing?.let {
            Surface(color = MaterialTheme.colorScheme.tertiaryContainer, modifier = Modifier.fillMaxWidth()) {
                Row(Modifier.padding(start = 16.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text(stringResource(R.string.signature_place), style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                    TextButton(onClick = { placing = null }) { Text(stringResource(R.string.cancel)) }
                }
            }
        }
        if (vm.readingPage >= 0 || vm.readAloudProblem != null) {
            Surface(color = MaterialTheme.colorScheme.secondaryContainer, modifier = Modifier.fillMaxWidth()) {
                Row(Modifier.padding(start = 16.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        when (vm.readAloudProblem) {
                            ReadAloud.Problem.NoEngine -> stringResource(R.string.read_aloud_no_engine)
                            ReadAloud.Problem.NoVoice -> stringResource(R.string.read_aloud_no_voice)
                            null -> stringResource(R.string.read_aloud_page, vm.readingPage + 1)
                        },
                        style = MaterialTheme.typography.bodyMedium,
                        modifier = Modifier.weight(1f),
                    )
                    TextButton(onClick = {
                        vm.stopReading()
                        vm.dismissReadAloudProblem()
                    }) { Text(stringResource(if (vm.readAloudProblem != null) R.string.close else R.string.stop)) }
                }
            }
        }
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
            // As sharp as the zoom asks, within MAX_RENDER_WIDTH and the memory one page may take.
            val tallest = remember(state) { state.pages.maxOf { (w, h) -> h / w }.coerceAtLeast(0.1f) }
            val byMemory = sqrt(vm.maxPageBytes / (4.0 * tallest)).toInt()
            val renderPx = minOf((widthPx * zoom).roundToInt(), maxOf(widthPx, MAX_RENDER_WIDTH), byMemory)
            val heightPx = constraints.maxHeight.toFloat()
            val contentWidth = maxWidth * zoom
            // Bring the current match into view, a third of the way down the screen.
            LaunchedEffect(vm.currentMatch, vm.found) {
                val hit = vm.found.firstOrNull { it.index == vm.currentMatch } ?: return@LaunchedEffect
                val (pw, _) = state.pages.getOrNull(hit.page) ?: return@LaunchedEffect
                val pxPerPt = widthPx * zoom / pw
                list.animateScrollToItem(hit.page, (hit.top * pxPerPt - heightPx / 3).roundToInt().coerceAtLeast(0))
                hScroll.animateScrollTo((hit.left * pxPerPt - widthPx / 3f).roundToInt().coerceAtLeast(0))
            }
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
                        val signature = placing
                        val tapToSign = if (signature != null && pdf != null) {
                            Modifier.pointerInput(signature) {
                                detectTapGestures(onTap = { at ->
                                    val k = w / size.width
                                    pdf.sign(i, placeSignature(signature, Offset(at.x * k, at.y * k), w, h))
                                    placing = null
                                })
                            }
                        } else {
                            Modifier
                        }
                        Box(Modifier.fillMaxWidth().aspectRatio(w / h).background(paper).then(tapToSign)) {
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
                            hitsByPage[i]?.let { hits ->
                                val current = vm.currentMatch
                                Canvas(Modifier.fillMaxSize()) {
                                    val k = size.width / w
                                    for (h in hits) {
                                        drawRect(
                                            if (h.index == current) currentHitColor else hitColor,
                                            topLeft = Offset(h.left * k, h.top * k),
                                            size = Size((h.right - h.left) * k, (h.bottom - h.top) * k),
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        BottomAppBar {
            // Scrolls sideways when the buttons do not fit (narrow phones, large text).
            Row(Modifier.horizontalScroll(rememberScrollState())) {
                TextButton(onClick = { showReading = true }) { Text(stringResource(R.string.reading)) }
                TextButton(onClick = { showPages = true }) { Text(stringResource(R.string.pages)) }
                if (pdf != null) {
                    TextButton(onClick = { drawSignature = true }, enabled = !pdf.busy && !pdf.restricted) { Text(stringResource(R.string.sign)) }
                    TextButton(onClick = { showPassword = true }, enabled = !pdf.busy && !pdf.restricted) { Text(stringResource(R.string.password)) }
                    TextButton(onClick = { showCompress = true }, enabled = !pdf.busy) { Text(stringResource(R.string.compress)) }
                }
                if (vm.exportFormats.isNotEmpty()) {
                    TextButton(onClick = { showConvert = true }) { Text(stringResource(R.string.convert)) }
                }
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
    }

    if (showCompress && pdf != null) {
        ModalBottomSheet(onDismissRequest = { showCompress = false }) {
            Text(
                stringResource(R.string.compress_title),
                style = MaterialTheme.typography.titleLarge,
                modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
            )
            for ((forScreen, title, hint) in listOf(
                Triple(false, R.string.compress_standard, R.string.compress_standard_hint),
                Triple(true, R.string.compress_screen, R.string.compress_screen_hint),
            )) {
                ListItem(
                    headlineContent = { Text(stringResource(title)) },
                    supportingContent = { Text(stringResource(hint)) },
                    modifier = Modifier.clickable {
                        showCompress = false
                        pdf.compress(forScreen)
                    },
                )
            }
            Text(
                stringResource(R.string.compress_note),
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
            )
            Spacer(Modifier.height(24.dp))
        }
    }

    if (showPassword && pdf != null) {
        if (pdf.isProtected) {
            AlertDialog(
                onDismissRequest = { showPassword = false },
                title = { Text(stringResource(R.string.unprotect_title)) },
                text = { Text(stringResource(R.string.unprotect_body)) },
                confirmButton = {
                    TextButton(onClick = {
                        showPassword = false
                        pdf.unprotect()
                    }) { Text(stringResource(R.string.unprotect)) }
                },
                dismissButton = { TextButton(onClick = { showPassword = false }) { Text(stringResource(R.string.cancel)) } },
            )
        } else {
            PasswordDialog(
                title = stringResource(R.string.protect_title),
                message = stringResource(R.string.protect_body),
                problem = null,
                confirm = stringResource(R.string.protect),
                onDismiss = { showPassword = false },
                onConfirm = {
                    showPassword = false
                    pdf.protect(it)
                },
            )
        }
    }

    if (drawSignature) {
        SignaturePad(SavedSignatures(File(context.filesDir, "signatures.txt")), onDismiss = { drawSignature = false }) {
            drawSignature = false
            placing = it
        }
    }

    if (showReading) {
        ModalBottomSheet(onDismissRequest = { showReading = false }) {
            Text(
                stringResource(R.string.reading),
                style = MaterialTheme.typography.titleLarge,
                modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
            )
            if (vm.canReadAloud) {
                ListItem(
                    headlineContent = { Text(stringResource(R.string.read_aloud)) },
                    supportingContent = { Text(stringResource(R.string.read_aloud_hint)) },
                    modifier = Modifier.clickable {
                        showReading = false
                        vm.startReading(current - 1)
                    },
                )
            }
            SettingRow(stringResource(R.string.night_mode), stringResource(R.string.night_mode_hint), reading.night, reading::updateNight)
            SettingRow(stringResource(R.string.keep_screen_on), null, reading.keepScreenOn, reading::updateKeepScreenOn)
            SettingRow(stringResource(R.string.volume_keys), stringResource(R.string.volume_keys_hint), reading.volumeKeys, reading::updateVolumeKeys)
            Spacer(Modifier.height(24.dp))
        }
    }

    if (showConvert) {
        ModalBottomSheet(onDismissRequest = { showConvert = false }) {
            Text(
                stringResource(R.string.convert_to),
                style = MaterialTheme.typography.titleLarge,
                modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
            )
            for (format in vm.exportFormats) {
                ListItem(
                    headlineContent = { Text(format.label) },
                    supportingContent = { Text(convertedName(state.fileName, format.ext)) },
                    modifier = Modifier.clickable {
                        showConvert = false
                        converting = format
                        createFile.launch(CreateFile.Request(convertedName(state.fileName, format.ext), format.mime))
                    },
                )
            }
            Spacer(Modifier.height(24.dp))
        }
    }

    if (showPages) {
        ModalBottomSheet(onDismissRequest = { showPages = false }) {
            Row(Modifier.fillMaxWidth().padding(horizontal = 24.dp), verticalAlignment = Alignment.CenterVertically) {
                Text(stringResource(R.string.pages), style = MaterialTheme.typography.titleLarge, modifier = Modifier.weight(1f))
                if (vm.canEditPages) {
                    TextButton(onClick = {
                        selecting = !selecting
                        selected.clear()
                    }) { Text(stringResource(if (selecting) R.string.done else R.string.select)) }
                }
                if (!selecting) {
                    TextButton(onClick = {
                        showPages = false
                        showGoTo = true
                    }) { Text(stringResource(R.string.go_to_page)) }
                }
            }
            if (selecting && pdf != null) {
                val picked = selected.sorted()
                Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 16.dp)) {
                    TextButton(onClick = { pdf.rotatePages(picked, -90) }, enabled = picked.isNotEmpty()) { Text("⟲ " + stringResource(R.string.rotate_left)) }
                    TextButton(onClick = { pdf.rotatePages(picked, 90) }, enabled = picked.isNotEmpty()) { Text("⟳ " + stringResource(R.string.rotate_right)) }
                    TextButton(onClick = {
                        pdf.deletePages(picked)
                        selected.clear()
                    }, enabled = picked.isNotEmpty() && picked.size < state.pages.size) { Text(stringResource(R.string.delete)) }
                    TextButton(onClick = {
                        extractPages = picked
                        extractFile.launch(CreateFile.Request(editedName.replace("-duzenlendi", "-sayfalar"), "application/pdf"))
                    }, enabled = picked.isNotEmpty()) { Text(stringResource(R.string.extract)) }
                    TextButton(onClick = { addPdf.launch(arrayOf("application/pdf")) }) { Text(stringResource(R.string.add_pdf)) }
                }
                Text(
                    pluralStringResource(R.plurals.pages_selected, picked.size, picked.size),
                    style = MaterialTheme.typography.labelMedium,
                    modifier = Modifier.padding(horizontal = 24.dp),
                )
            }
            val grid = rememberLazyGridState(initialFirstVisibleItemIndex = (current - 1).coerceAtLeast(0))
            LazyVerticalGrid(
                columns = GridCells.Adaptive(96.dp),
                state = grid,
                contentPadding = PaddingValues(16.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                items(state.pages.size, key = { it }) { i ->
                    val (w, h) = state.pages[i]
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        modifier = Modifier.clickable {
                            if (selecting) {
                                if (i in selected) selected.remove(i) else selected.add(i)
                            } else {
                                showPages = false
                                scope.launch { list.scrollToItem(i) }
                            }
                        },
                    ) {
                        Box(
                            Modifier
                                .fillMaxWidth()
                                .aspectRatio(w / h)
                                .background(if (invertOnScreen) Color.Black else Color.White)
                                .border(
                                    if (i in selected) 4.dp else if (!selecting && i + 1 == current) 3.dp else 1.dp,
                                    when {
                                        i in selected -> MaterialTheme.colorScheme.tertiary
                                        !selecting && i + 1 == current -> MaterialTheme.colorScheme.primary
                                        else -> Color(0x33000000)
                                    },
                                ),
                        ) {
                            vm.thumbnail(i)?.let { bmp ->
                                Image(
                                    bmp.asImageBitmap(),
                                    contentDescription = null,
                                    contentScale = ContentScale.FillWidth,
                                    colorFilter = if (invertOnScreen) invertColors else null,
                                    modifier = Modifier.fillMaxSize(),
                                )
                            }
                        }
                        Text("${i + 1}", style = MaterialTheme.typography.labelMedium, modifier = Modifier.padding(top = 4.dp))
                    }
                }
            }
        }
    }

    if (vm.askClose) {
        AlertDialog(
            onDismissRequest = { vm.answerClose(null) },
            title = { Text(stringResource(R.string.unsaved_title)) },
            text = { Text(stringResource(R.string.unsaved_body)) },
            confirmButton = {
                TextButton(onClick = {
                    vm.answerClose(null)
                    closeAfterSave = true
                    saveEdited.launch(CreateFile.Request(editedName, "application/pdf"))
                }) { Text(stringResource(R.string.save)) }
            },
            dismissButton = {
                Row {
                    TextButton(onClick = { vm.answerClose(null) }) { Text(stringResource(R.string.cancel)) }
                    TextButton(onClick = { vm.answerClose(true) }) { Text(stringResource(R.string.discard)) }
                }
            },
        )
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
