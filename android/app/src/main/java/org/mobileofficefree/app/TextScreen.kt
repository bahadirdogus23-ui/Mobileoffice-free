package org.mobileofficefree.app

import android.content.Intent
import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.BottomAppBar
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

private val hitColor = Color(0x66FFD54F)
private val currentHitColor = Color(0xFFFF9800)

/** Plain text with line numbers: search (Turkish-letter insensitive), text size, wrapping, night colours. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TextScreen(vm: TextViewModel, state: TextUiState, reading: ReadingSettings) {
    val context = LocalContext.current
    val list = rememberLazyListState()
    var fontSize by rememberSaveable { mutableFloatStateOf(14f) }
    var wrap by rememberSaveable { mutableStateOf(true) }
    var searching by remember(state.uri) { mutableStateOf(false) }
    val night = reading.night
    val paper = if (night) Color(0xFF1B1B1B) else Color.White
    val ink = if (night) Color(0xFFE0E0E0) else Color(0xFF1B1B1B)
    val gutterInk = if (night) Color(0xFF8A8A8A) else Color(0xFF9E9E9E)
    val style = TextStyle(fontFamily = FontFamily.Monospace, fontSize = fontSize.sp, color = ink)
    val digits = state.lineCount.toString().length
    val matchesByLine = remember(vm.matches) { vm.matches.withIndex().groupBy({ it.value.line }, { it.index to it.value }) }

    // Bring the current hit into view.
    LaunchedEffect(vm.currentMatch, vm.matches) {
        vm.matches.getOrNull(vm.currentMatch)?.let { list.scrollToItem((it.line - 2).coerceAtLeast(0)) }
    }

    Column(Modifier.fillMaxSize()) {
        TopAppBar(
            title = {
                Column {
                    Text(state.fileName, maxLines = 1)
                    Text(
                        stringResource(R.string.text_info, state.encoding, state.lineCount),
                        style = MaterialTheme.typography.bodySmall,
                    )
                }
            },
            navigationIcon = {
                IconButton(onClick = vm::close) {
                    Icon(painterResource(R.drawable.ic_arrow_back), contentDescription = stringResource(R.string.back))
                }
            },
            actions = {
                TextButton(onClick = { fontSize = (fontSize - 2f).coerceAtLeast(8f) }) { Text("A−") }
                TextButton(onClick = { fontSize = (fontSize + 2f).coerceAtMost(32f) }) { Text("A+") }
            },
        )
        if (state.truncated) {
            Surface(color = MaterialTheme.colorScheme.tertiaryContainer, modifier = Modifier.fillMaxWidth()) {
                Text(
                    stringResource(R.string.text_truncated, TextViewModel.MAX_BYTES / (1024 * 1024)),
                    style = MaterialTheme.typography.bodySmall,
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 6.dp),
                )
            }
        }
        if (searching) {
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
                        vm.query.isEmpty() -> ""
                        vm.matches.isEmpty() -> stringResource(R.string.no_matches)
                        else -> stringResource(R.string.page_counter, vm.currentMatch + 1, vm.matches.size)
                    },
                    style = MaterialTheme.typography.labelMedium,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.widthIn(min = 56.dp).padding(horizontal = 4.dp),
                )
                TextButton(onClick = { vm.nextMatch(-1) }, enabled = vm.matches.isNotEmpty()) { Text("▲") }
                TextButton(onClick = { vm.nextMatch(1) }, enabled = vm.matches.isNotEmpty()) { Text("▼") }
            }
        }
        val gutterWidth = with(LocalDensity.current) {
            rememberTextMeasurer().measure("0".repeat(digits), style).size.width.toDp() + 12.dp
        }
        val charWidth = with(LocalDensity.current) { rememberTextMeasurer().measure("0", style).size.width.toDp() }
        val longest = remember(state) { state.lines.maxOfOrNull { it.text.length } ?: 0 }
        val hScroll = rememberScrollState()
        Box(Modifier.weight(1f).fillMaxWidth().background(paper)) {
            SelectionContainer {
                LazyColumn(
                    state = list,
                    modifier = if (wrap) {
                        Modifier.fillMaxSize()
                    } else {
                        Modifier.fillMaxHeight().horizontalScroll(hScroll).width(gutterWidth + charWidth * (longest + 2))
                    },
                ) {
                    items(state.lines.size) { i ->
                        val line = state.lines[i]
                        Row(Modifier.fillMaxWidth().padding(end = 8.dp)) {
                            Text(
                                if (line.continued) "" else line.number.toString(),
                                style = style.copy(color = gutterInk),
                                textAlign = TextAlign.End,
                                modifier = Modifier.width(gutterWidth).padding(end = 8.dp),
                            )
                            Text(highlight(line.text, matchesByLine[i], vm.currentMatch), style = style, softWrap = wrap)
                        }
                    }
                }
            }
        }
        BottomAppBar {
            TextButton(onClick = { searching = !searching; if (!searching) vm.search("") }) { Text(stringResource(R.string.search)) }
            TextButton(onClick = { wrap = !wrap }) {
                Text(stringResource(if (wrap) R.string.wrap_off else R.string.wrap_on))
            }
            TextButton(onClick = { reading.updateNight(!night) }) { Text(stringResource(if (night) R.string.day_short else R.string.night_short)) }
            TextButton(onClick = {
                val send = Intent(Intent.ACTION_SEND)
                    .setType(context.contentResolver.getType(state.uri) ?: "text/plain")
                    .putExtra(Intent.EXTRA_STREAM, state.uri)
                    .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                context.startActivity(Intent.createChooser(send, state.fileName))
            }) { Text(stringResource(R.string.share)) }
        }
    }
}

private fun highlight(text: String, hits: List<Pair<Int, TextMatch>>?, current: Int): AnnotatedString {
    if (hits.isNullOrEmpty()) return AnnotatedString(text)
    return buildAnnotatedString {
        append(text)
        for ((index, m) in hits) {
            addStyle(SpanStyle(background = if (index == current) currentHitColor else hitColor), m.start, m.end)
        }
    }
}
