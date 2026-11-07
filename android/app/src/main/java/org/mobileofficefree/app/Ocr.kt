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

    /** The text in [bitmap], lines kept as lines; blank when none was found. */
    suspend fun recognize(context: Context, bitmap: Bitmap): String = lock.withLock {
        withContext(Dispatchers.Default) {
            val started = SystemClock.elapsedRealtime()
            val api = TessBaseAPI()
            try {
                if (!api.init(dataDir(context).absolutePath, LANG)) error("OCR model could not be loaded")
                api.setPageSegMode(TessBaseAPI.PageSegMode.PSM_AUTO)
                api.setImage(bitmap)
                api.getUTF8Text().orEmpty().trim().also {
                    Measurements.sample("OCR: tanıma (${bitmap.width}×${bitmap.height})", SystemClock.elapsedRealtime() - started)
                    Diagnostics.event("OCR: ${it.length} karakter, ${api.meanConfidence()} güven")
                }
            } finally {
                api.recycle()
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
