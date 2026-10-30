package org.mobileofficefree.app

import android.content.Intent
import android.text.format.Formatter
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.BottomAppBar
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.viewModelScope

private const val MAX_IMAGE_ZOOM = 8f

/** A picture: pinch to zoom, drag to move, double tap to zoom in or back; rotate, print, share. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ImageScreen(vm: ImageViewModel, state: ImageUiState) {
    val context = LocalContext.current
    var scale by remember(state.uri) { mutableFloatStateOf(1f) }
    var shift by remember(state.uri) { mutableStateOf(Offset.Zero) }
    var quarterTurns by remember(state.uri) { mutableIntStateOf(0) }
    val bitmap = remember(state.bitmap) { state.bitmap.asImageBitmap() }
    BackHandler { vm.close() }

    Column(Modifier.fillMaxSize()) {
        TopAppBar(
            title = {
                Column {
                    Text(state.fileName.substringBeforeLast('.'), maxLines = 1)
                    val size = if (state.size >= 0) " · " + Formatter.formatShortFileSize(context, state.size) else ""
                    Text("${state.width} × ${state.height}$size", style = MaterialTheme.typography.bodySmall)
                }
            },
            navigationIcon = {
                IconButton(onClick = vm::close) {
                    Icon(painterResource(R.drawable.ic_arrow_back), contentDescription = stringResource(R.string.back))
                }
            },
        )
        BoxWithConstraints(
            Modifier
                .weight(1f)
                .fillMaxWidth()
                .clipToBounds()
                .background(Color.Black),
        ) {
            val w = constraints.maxWidth.toFloat()
            val h = constraints.maxHeight.toFloat()
            val center = Offset(w / 2, h / 2)
            // A quarter turn swaps the picture's sides; shrink it so the turned picture still fits.
            val fit = if (quarterTurns % 2 == 0) {
                1f
            } else {
                val k = minOf(w / state.bitmap.width, h / state.bitmap.height)
                minOf(w / (state.bitmap.height * k), h / (state.bitmap.width * k), 1f)
            }
            // Zoom about a screen point [p] to [next], keeping the picture point under it in place.
            fun zoomAt(p: Offset, next: Float) {
                val s = next.coerceIn(1f, MAX_IMAGE_ZOOM)
                shift = if (s == 1f) Offset.Zero else p - center - (p - center - shift) * (s / scale)
                scale = s
            }
            Image(
                bitmap,
                contentDescription = state.fileName,
                contentScale = ContentScale.Fit,
                modifier = Modifier
                    .fillMaxSize()
                    .pointerInput(state.uri, w, h) {
                        detectTransformGestures { centroid, pan, zoom, _ ->
                            zoomAt(centroid, scale * zoom)
                            if (scale > 1f) shift += pan
                        }
                    }
                    .pointerInput(state.uri, w, h) {
                        detectTapGestures(onDoubleTap = { p -> zoomAt(p, if (scale > 1.01f) 1f else 2.5f) })
                    }
                    .graphicsLayer {
                        scaleX = scale * fit
                        scaleY = scale * fit
                        rotationZ = 90f * quarterTurns
                        translationX = shift.x
                        translationY = shift.y
                    },
            )
        }
        BottomAppBar {
            TextButton(onClick = {
                quarterTurns = (quarterTurns + 1) % 4
                scale = 1f
                shift = Offset.Zero
            }) { Text(stringResource(R.string.rotate)) }
            TextButton(onClick = { DocumentPrinter.printImage(context, vm.viewModelScope, state.fileName, state.bitmap) }) {
                Text(stringResource(R.string.print))
            }
            TextButton(onClick = {
                val send = Intent(Intent.ACTION_SEND)
                    .setType(context.contentResolver.getType(state.uri) ?: "image/*")
                    .putExtra(Intent.EXTRA_STREAM, state.uri)
                    .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                context.startActivity(Intent.createChooser(send, state.fileName))
            }) { Text(stringResource(R.string.share)) }
        }
    }
}
