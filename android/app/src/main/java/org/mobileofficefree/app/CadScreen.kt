package org.mobileofficefree.app

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
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
import androidx.compose.material3.TextButton
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
            actions = {
                TextButton(onClick = vm::toggleBackground) {
                    Text(stringResource(if (state.dark) R.string.cad_white_sheet else R.string.cad_dark_sheet))
                }
                TextButton(onClick = { showLayers = true }) { Text(stringResource(R.string.cad_layers)) }
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
                .pointerInput(state.uri) { detectTapGestures(onDoubleTap = { vm.fit() }) },
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
        }
        if (state.spaces.size > 1) {
            PrimaryScrollableTabRow(selectedTabIndex = state.space) {
                state.spaces.forEachIndexed { i, name ->
                    Tab(selected = i == state.space, onClick = { vm.selectSpace(i) }, text = { Text(name) })
                }
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
