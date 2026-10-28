package org.mobileofficefree.app

import android.app.Application
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.ImageDecoder
import android.net.Uri
import android.os.Build
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** A picture and its size in the file (it may be shown smaller). */
data class ImageUiState(val uri: Uri, val fileName: String, val bitmap: Bitmap, val width: Int, val height: Int, val size: Long)

/** Pictures (JPEG, PNG, WebP, GIF, BMP, HEIF, AVIF) through Android's own decoders. */
class ImageViewModel(app: Application) : AndroidViewModel(app) {
    private val recents = RecentFiles(app)

    var state by mutableStateOf<ImageUiState?>(null)
        private set
    var loading by mutableStateOf(false)
        private set
    var error by mutableStateOf<String?>(null)
        private set

    fun open(uri: Uri) {
        val resolver = getApplication<Application>().contentResolver
        loading = true
        viewModelScope.launch {
            val result = withContext(Dispatchers.IO) {
                runCatching {
                    val info = FileInfo.of(resolver, uri)
                    val (bitmap, w, h) = decode(uri)
                    recents.remember(uri, info.name, DocumentKind.IMAGE, info.size)
                    ImageUiState(uri, info.name, bitmap, w, h, info.size)
                }
            }
            loading = false
            result.fold(onSuccess = { state = it }, onFailure = { error = it.message ?: it.javaClass.simpleName })
        }
    }

    fun close() {
        state = null
    }

    fun consumeError() {
        error = null
    }

    /** Decodes upright (EXIF rotation applied where the platform does it) and at most [MAX_SIDE] pixels a side. */
    private fun decode(uri: Uri): Triple<Bitmap, Int, Int> {
        val resolver = getApplication<Application>().contentResolver
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            var w = 0
            var h = 0
            val bmp = ImageDecoder.decodeBitmap(ImageDecoder.createSource(resolver, uri)) { decoder, info, _ ->
                w = info.size.width
                h = info.size.height
                decoder.setTargetSampleSize(sampleSize(maxOf(w, h)))
                // Software memory, so the picture can also be drawn into a print job.
                decoder.allocator = ImageDecoder.ALLOCATOR_SOFTWARE
            }
            return Triple(bmp, w, h)
        }
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        resolver.openInputStream(uri)?.use { BitmapFactory.decodeStream(it, null, bounds) }
        val opts = BitmapFactory.Options().apply { inSampleSize = sampleSize(maxOf(bounds.outWidth, bounds.outHeight)) }
        val bmp = resolver.openInputStream(uri)?.use { BitmapFactory.decodeStream(it, null, opts) } ?: error("not a picture")
        return Triple(bmp, bounds.outWidth, bounds.outHeight)
    }

    private fun sampleSize(side: Int): Int {
        var s = 1
        while (side / s > MAX_SIDE) s *= 2
        return s
    }

    private companion object {
        /** Big enough to zoom into, small enough for every phone's memory (4096² × 4 bytes = 64 MB at most). */
        const val MAX_SIDE = 4096
    }
}
