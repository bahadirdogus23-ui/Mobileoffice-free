package org.mobileofficefree.app

import android.content.Intent
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.BottomAppBar
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.flow.debounce
import kotlinx.coroutines.FlowPreview

/** Word and PDF documents: pages one under another, fitted to the screen width, with a page counter. */
@OptIn(ExperimentalMaterial3Api::class, FlowPreview::class)
@Composable
fun PagedScreen(vm: PagedViewModel, state: PagedUiState) {
    val context = LocalContext.current
    val list = rememberLazyListState()
    val current by remember { derivedStateOf { list.firstVisibleItemIndex + 1 } }
    LaunchedEffect(state.uri) {
        snapshotFlow { list.firstVisibleItemIndex }.debounce(500).collect(vm::reportPosition)
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
                Text(
                    stringResource(R.string.page_counter, current, state.pages.size),
                    color = MaterialTheme.colorScheme.primary,
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(end = 16.dp),
                )
            },
        )
        BoxWithConstraints(Modifier.weight(1f).fillMaxWidth().background(Color(0xFF3A3A3A))) {
            val widthPx = with(LocalDensity.current) { maxWidth.roundToPx() }
            LazyColumn(state = list, verticalArrangement = Arrangement.spacedBy(8.dp)) {
                items(state.pages.size, key = { it }) { i ->
                    val (w, h) = state.pages[i]
                    Box(Modifier.fillMaxWidth().aspectRatio(w / h).background(Color.White)) {
                        val bmp = vm.page(i, widthPx)
                        if (bmp != null) {
                            Image(
                                bmp.asImageBitmap(),
                                contentDescription = stringResource(R.string.page_counter, i + 1, state.pages.size),
                                contentScale = ContentScale.FillWidth,
                                modifier = Modifier.fillMaxSize(),
                            )
                        }
                    }
                }
            }
        }
        BottomAppBar {
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
