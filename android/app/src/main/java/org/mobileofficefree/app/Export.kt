package org.mobileofficefree.app

import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.activity.result.contract.ActivityResultContract

/** A format a document can be converted to: its extension, MIME type and menu label. */
data class ExportFormat(val ext: String, val mime: String, val label: String)

val PDF_FORMAT = ExportFormat("pdf", "application/pdf", "PDF")

/** Word documents can become any of these; WordCraft writes them all. */
val WORD_FORMATS = listOf(
    PDF_FORMAT,
    ExportFormat("docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document", "Word (DOCX)"),
    ExportFormat("odt", "application/vnd.oasis.opendocument.text", "OpenDocument (ODT)"),
    ExportFormat("rtf", "application/rtf", "RTF"),
    ExportFormat("txt", "text/plain", "TXT"),
)

/**
 * The system "Save as" screen for a new file: [Request.name] suggested, of type [Request.mime].
 * (The stock CreateDocument contract fixes the type when the screen is built; here each
 * conversion picks its own.)
 */
class CreateFile : ActivityResultContract<CreateFile.Request, Uri?>() {
    data class Request(val name: String, val mime: String)

    override fun createIntent(context: Context, input: Request): Intent =
        Intent(Intent.ACTION_CREATE_DOCUMENT)
            .addCategory(Intent.CATEGORY_OPENABLE)
            .setType(input.mime)
            .putExtra(Intent.EXTRA_TITLE, input.name)

    override fun parseResult(resultCode: Int, intent: Intent?): Uri? = intent?.data
}

/** [fileName] with its extension changed to [ext]: "Rapor.doc" → "Rapor.pdf". */
fun convertedName(fileName: String, ext: String): String = fileName.substringBeforeLast('.') + "." + ext
