package org.mobileofficefree.app

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/**
 * The document's text flowing to the screen's width, as an e-book reader shows it: no zooming or
 * sideways scrolling, a text size the person picks, dark or light, and text that can be selected
 * and copied. Starts at [startPage].
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ReflowScreen(vm: PagedViewModel, title: String, startPage: Int, reading: ReadingSettings, onClose: () -> Unit) {
    val pages by produceState<List<String>?>(null) { value = vm.textPages() }
    val list = rememberLazyListState()
    val dark = reading.night
    val paper = if (dark) Color(0xFF121212) else Color(0xFFFBF8F1)
    val ink = if (dark) Color(0xFFE6E1D8) else Color(0xFF222222)
    BackHandler(onBack = onClose)
    Column(Modifier.fillMaxSize().background(paper).navigationBarsPadding()) {
        TopAppBar(
            title = { Text(title, maxLines = 1) },
            navigationIcon = { TextButton(onClick = onClose) { Text(stringResource(R.string.pages_view)) } },
            actions = {
                TextButton(onClick = { reading.updateTextSize(reading.textSize - 2f) }) { Text("A−") }
                TextButton(onClick = { reading.updateTextSize(reading.textSize + 2f) }) { Text("A+") }
            },
        )
        val all = pages
        when {
            all == null -> Column(Modifier.fillMaxSize(), horizontalAlignment = Alignment.CenterHorizontally) { CircularProgressIndicator(Modifier.padding(48.dp)) }
            all.all { it.isBlank() } -> Text(
                stringResource(R.string.reflow_no_text),
                color = ink,
                textAlign = TextAlign.Center,
                modifier = Modifier.fillMaxWidth().padding(32.dp),
            )
            else -> {
                LaunchedEffect(all) { if (startPage in all.indices) list.scrollToItem(startPage) }
                SelectionContainer {
                    LazyColumn(state = list, modifier = Modifier.fillMaxSize()) {
                        itemsIndexed(all) { i, text ->
                            Column(Modifier.padding(horizontal = 20.dp, vertical = 8.dp)) {
                                Row {
                                    Text(
                                        stringResource(R.string.page_counter, i + 1, all.size),
                                        color = ink.copy(alpha = 0.5f),
                                        style = MaterialTheme.typography.labelMedium,
                                    )
                                }
                                Text(
                                    text.trim(),
                                    color = ink,
                                    fontSize = reading.textSize.sp,
                                    lineHeight = (reading.textSize * 1.5f).sp,
                                    modifier = Modifier.padding(top = 4.dp, bottom = 12.dp),
                                )
                                HorizontalDivider(color = ink.copy(alpha = 0.12f))
                            }
                        }
                    }
                }
            }
        }
    }
}
