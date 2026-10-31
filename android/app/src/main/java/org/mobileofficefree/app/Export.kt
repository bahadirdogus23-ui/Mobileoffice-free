package org.mobileofficefree.app

import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.activity.result.contract.ActivityResultContract

/** A format a document can be converted to: its extension, MIME type and menu label. */
data class ExportFormat(val ext: String, val mime: String, val label: String)

val PDF_FORMAT = ExportFormat("pdf", "application/pdf", "PDF")

/** What a PDF can become: its text, or a picture of each page. */
val PDF_EXPORT_FORMATS = listOf(
    ExportFormat("txt", "text/plain", "TXT"),
    ExportFormat("zip", "application/zip", "JPEG (ZIP)"),
)

/** Word documents can become any of these; WordCraft writes them all. */
val WORD_FORMATS = listOf(
    PDF_FORMAT,
    ExportFormat("docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document", "Word (DOCX)"),
    ExportFormat("odt", "application/vnd.oasis.opendocument.text", "OpenDocument (ODT)"),
    ExportFormat("rtf", "application/rtf", "RTF"),
    ExportFormat("txt", "text/plain", "TXT"),
)

/** Plain text can become any of these, through WordCraft (each line a paragraph). */
val TEXT_FORMATS = listOf(
    PDF_FORMAT,
    ExportFormat("docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document", "Word (DOCX)"),
    ExportFormat("html", "text/html", "HTML"),
    ExportFormat("md", "text/markdown", "Markdown"),
    ExportFormat("odt", "application/vnd.oasis.opendocument.text", "OpenDocument (ODT)"),
    ExportFormat("rtf", "application/rtf", "RTF"),
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

/** Paper sizes offered for text documents, in points. */
enum class Paper(val label: String, val widthPt: Float, val heightPt: Float) {
    A4("A4", 595.28f, 841.89f),
    A5("A5", 419.53f, 595.28f),
    A3("A3", 841.89f, 1190.55f),
    LETTER("Letter", 612f, 792f),
}

/** Fonts bundled with the engine (OFL), so the result looks the same on every phone. */
enum class TextFont(val family: String) {
    SANS("Source Sans 3"),
    SERIF("Source Serif 4"),
    MONO("JetBrains Mono"),
}

/** How plain text is laid out when it becomes a PDF or a word-processing document. */
data class TextPageSetup(
    val paper: Paper = Paper.A4,
    val landscape: Boolean = false,
    /** 36 pt = 1.27 cm (narrow), 72 pt = 2.54 cm (normal), 108 pt = 3.81 cm (wide). */
    val marginPt: Float = 72f,
    val font: TextFont = TextFont.SANS,
    val sizePt: Float = 11f,
)

/** Formats with pages, which ask for a [TextPageSetup] first. */
fun ExportFormat.hasPages(): Boolean = ext in setOf("pdf", "docx", "odt", "rtf")
