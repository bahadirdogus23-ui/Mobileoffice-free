package org.mobileofficefree.app

import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.net.Uri
import android.os.SystemClock
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalResources
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.googlecode.tesseract.android.TessBaseAPI
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import kotlin.math.roundToInt

/**
 * Text recognition on the phone (Tesseract with its Turkish model, bundled in the app; no
 * network, no Google services: ADR-0006). One recognition at a time.
 */
object Ocr {
    private const val LANG = "tur"
    private val lock = Mutex()

    /** The folder holding `tessdata/tur.traineddata`, copied out of the app's assets once. */
    private fun dataDir(context: Context): File {
        val root = File(context.filesDir, "ocr")
        val model = File(root, "tessdata/$LANG.traineddata")
        val size = context.assets.openFd("tessdata/$LANG.traineddata").use { it.length }
        if (!model.isFile || model.length() != size) {
            model.parentFile?.mkdirs()
            val tmp = File(model.parentFile, "$LANG.tmp")
            context.assets.open("tessdata/$LANG.traineddata").use { input -> tmp.outputStream().use { input.copyTo(it) } }
            tmp.renameTo(model)
        }
        return root
    }

    /**
     * What Tesseract reads in [bitmap], after [OcrImage] has prepared it: the lines with their
     * words' confidences and boxes (pixels of the prepared picture), in reading order, and the
     * prepared picture's size. Runs on the caller's thread (never the main one).
     */
    private fun read(context: Context, bitmap: Bitmap): Triple<List<OcrReadLine>, Int, Int> {
        val prepared = prepare(bitmap)
        val api = TessBaseAPI()
        try {
            if (!api.init(dataDir(context).absolutePath, LANG)) error("OCR model could not be loaded")
            // PSM_AUTO_OSD would need osd.traineddata, which the app does not carry: plain
            // automatic layout. The prepared picture has no DPI of its own; 300 is what Tesseract
            // is tuned for, and it stops the "Invalid resolution" guess.
            api.setPageSegMode(TessBaseAPI.PageSegMode.PSM_AUTO)
            api.setVariable("user_defined_dpi", "300")
            api.setVariable("preserve_interword_spaces", "1")
            api.setImage(prepared)
            api.getUTF8Text()
            val lines = ArrayList<OcrReadLine>()
            val it = api.getResultIterator() ?: return Triple(emptyList(), prepared.width, prepared.height)
            try {
                it.begin()
                val word = TessBaseAPI.PageIteratorLevel.RIL_WORD
                var words = ArrayList<OcrReadWord>()
                var newBlock = false
                do {
                    // A new line (and maybe paragraph) starts: the words so far are one line.
                    if (it.isAtBeginningOf(TessBaseAPI.PageIteratorLevel.RIL_TEXTLINE)) {
                        if (words.isNotEmpty()) lines += OcrReadLine(words, newBlock)
                        words = ArrayList()
                        newBlock = it.isAtBeginningOf(TessBaseAPI.PageIteratorLevel.RIL_PARA)
                    }
                    val text = it.getUTF8Text(word)?.trim().orEmpty()
                    val r = it.getBoundingRect(word)
                    if (text.isNotEmpty() && r.width() > 0 && r.height() > 0) {
                        words += OcrReadWord(text, it.confidence(word), r.left, r.top, r.right, r.bottom)
                    }
                } while (it.next(word))
                if (words.isNotEmpty()) lines += OcrReadLine(words, newBlock)
            } finally {
                it.delete()
            }
            return Triple(lines, prepared.width, prepared.height)
        } finally {
            api.recycle()
            prepared.recycle()
        }
    }

    /** [bitmap] scaled ([OcrImage.scaleFactor]) and made black on white ([OcrImage.prepare]); a new bitmap. */
    private fun prepare(bitmap: Bitmap): Bitmap {
        val f = OcrImage.scaleFactor(bitmap.width, bitmap.height)
        val w = (bitmap.width * f).roundToInt().coerceAtLeast(1)
        val h = (bitmap.height * f).roundToInt().coerceAtLeast(1)
        // A hardware or 16-bit bitmap is copied first: its pixels cannot be read as they are.
        val readable = if (bitmap.config == Bitmap.Config.ARGB_8888) bitmap else bitmap.copy(Bitmap.Config.ARGB_8888, false)
        val scaled = if (w == readable.width && h == readable.height) readable else Bitmap.createScaledBitmap(readable, w, h, true)
        val pixels = IntArray(w * h)
        scaled.getPixels(pixels, 0, w, 0, 0, w, h)
        if (scaled !== bitmap) scaled.recycle()
        if (readable !== bitmap && readable !== scaled) readable.recycle()
        return Bitmap.createBitmap(OcrImage.prepare(pixels, w, h), w, h, Bitmap.Config.ARGB_8888)
    }

    /**
     * The words in [bitmap] with their boxes as fractions of it, for an invisible text layer.
     * Only words of lines that are text ([OcrLines.keep]); fractions of the prepared picture are
     * fractions of the original, as it is only scaled, never cropped.
     */
    suspend fun recognizeWords(context: Context, bitmap: Bitmap): List<org.mobileofficefree.engine.OcrWord> = lock.withLock {
        withContext(Dispatchers.Default) {
            val started = SystemClock.elapsedRealtime()
            val (lines, pw, ph) = read(context, bitmap)
            val w = pw.toDouble()
            val h = ph.toDouble()
            val out = OcrLines.kept(lines).flatMap { line ->
                line.words.map { org.mobileofficefree.engine.OcrWord(it.text, it.left / w, it.top / h, it.right / w, it.bottom / h) }
            }
            Measurements.sample("OCR: sayfa kelimeleri (${bitmap.width}×${bitmap.height})", SystemClock.elapsedRealtime() - started)
            out
        }
    }

    /** The text in [bitmap], lines kept as lines; blank when none was found. */
    suspend fun recognize(context: Context, bitmap: Bitmap): String = lock.withLock {
        withContext(Dispatchers.Default) {
            val started = SystemClock.elapsedRealtime()
            val (lines, _, _) = read(context, bitmap)
            val kept = OcrLines.kept(lines)
            val words = kept.flatMap { it.words }
            val confidence = if (words.isEmpty()) 0 else words.sumOf { it.confidence.toDouble() }.div(words.size).roundToInt()
            OcrLines.text(lines).trim().also {
                Measurements.sample("OCR: tanıma (${bitmap.width}×${bitmap.height})", SystemClock.elapsedRealtime() - started)
                Diagnostics.event("OCR: ${it.length} karakter, ${kept.size}/${lines.size} satır, $confidence güven")
            }
        }
    }
}

/**
 * "Recognise text": reads the text in the picture [source] gives (called off the main thread)
 * and shows it to select, copy, share or save as a text file. [onDone] closes it.
 */
@Composable
fun OcrDialog(name: String, source: suspend () -> Bitmap?, onDone: () -> Unit) {
    val context = LocalContext.current
    val resources = LocalResources.current
    val scope = rememberCoroutineScope()
    var text by remember { mutableStateOf<String?>(null) }
    var failed by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(Unit) {
        val result = runCatching {
            val bmp = withContext(Dispatchers.IO) { source() } ?: error("no picture")
            Ocr.recognize(context, bmp)
        }
        result.fold(onSuccess = { text = it }, onFailure = { failed = it.message ?: it.javaClass.simpleName })
    }
    val save = rememberLauncherForActivityResult(CreateFile()) { dest: Uri? ->
        val t = text ?: return@rememberLauncherForActivityResult
        if (dest != null) scope.launch {
            val ok = withContext(Dispatchers.IO) { runCatching { context.contentResolver.openOutputStream(dest, "wt")?.use { it.write(t.toByteArray()) } ?: error("cannot write") } }
            Toast.makeText(context, if (ok.isSuccess) resources.getString(R.string.saved) else resources.getString(R.string.save_failed, ok.exceptionOrNull()?.message ?: ""), Toast.LENGTH_SHORT).show()
        }
    }
    AlertDialog(
        onDismissRequest = onDone,
        title = { Text(stringResource(R.string.ocr_title)) },
        text = {
            val t = text
            when {
                failed != null -> Text(stringResource(R.string.open_failed, failed ?: ""))
                t == null -> Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    CircularProgressIndicator()
                    Text(stringResource(R.string.ocr_working))
                }
                t.isBlank() -> Text(stringResource(R.string.ocr_none))
                else -> Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(stringResource(R.string.ocr_note), style = MaterialTheme.typography.bodySmall)
                    SelectionContainer(Modifier.fillMaxWidth().heightIn(max = 360.dp).verticalScroll(rememberScrollState())) {
                        Text(t, style = MaterialTheme.typography.bodyMedium)
                    }
                    Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                        TextButton(onClick = {
                            context.getSystemService(android.content.ClipboardManager::class.java)?.setPrimaryClip(android.content.ClipData.newPlainText(name, t))
                        }) { Text(stringResource(R.string.copy)) }
                        TextButton(onClick = {
                            val send = Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, t)
                            context.startActivity(Intent.createChooser(send, name))
                        }) { Text(stringResource(R.string.share)) }
                        TextButton(onClick = { save.launch(CreateFile.Request(name.substringBeforeLast('.') + ".txt", "text/plain")) }) { Text(stringResource(R.string.ocr_save)) }
                    }
                }
            }
        },
        confirmButton = { TextButton(onClick = onDone) { Text(stringResource(R.string.close)) } },
    )
}
