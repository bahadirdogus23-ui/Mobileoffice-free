package org.mobileofficefree.app

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.widget.Toast
import androidx.core.content.FileProvider
import java.io.File
import java.util.Locale

/**
 * "Share" for texts and files, always through the system's chooser; the app itself never goes
 * online.
 *
 * Long texts (the measurements report holds up to 1500 kept lines) are cut before they go into
 * the intent: Android passes an intent's extras through a binder buffer of about 1 MB, the chooser
 * copies the text into its ClipData, and when that is too large the chooser finds no app at all
 * ("No apps can perform this action"). The whole text then goes along as a `.txt` file from the
 * app's own provider.
 */
object Share {
    /** About 100 KB as UTF-16; twice that with the chooser's copy, well under the binder limit. */
    const val MAX_TEXT_CHARS = 50_000

    /** Files handed out for sharing, in the provider's `reports/` path (res/xml/file_paths.xml). */
    private const val SHARED_TEXT_FILE = "BadoOffice-rapor.txt"

    /**
     * [text] cut to at most [max] characters, ending with [note] (on a line of its own), so the
     * reader sees it was shortened. Cut at a line end when one is near, never inside a surrogate
     * pair. Text that fits is returned as it is.
     */
    fun clipText(text: String, max: Int = MAX_TEXT_CHARS, note: String): String {
        if (text.length <= max) return text
        val room = (max - note.length - 2).coerceAtLeast(0)
        var end = room
        val newline = text.lastIndexOf('\n', (room - 1).coerceAtLeast(0))
        if (newline >= room / 2) end = newline
        if (end > 0 && Character.isHighSurrogate(text[end - 1])) end--
        return text.substring(0, end).trimEnd() + "\n\n" + note
    }

    /**
     * Hands [text] to the app the person picks. A text too long for an intent is cut, and the
     * whole of it attached as a `.txt` file ([fileName], in the app's cache).
     */
    fun text(context: Context, text: String, title: CharSequence? = null, fileName: String = SHARED_TEXT_FILE) {
        val send = Intent(Intent.ACTION_SEND).setType("text/plain")
        if (text.length <= MAX_TEXT_CHARS) {
            send.putExtra(Intent.EXTRA_TEXT, text)
        } else {
            val file = runCatching { textFile(context, text, fileName) }.getOrNull()
            val note = context.getString(if (file != null) R.string.share_text_cut else R.string.share_text_cut_no_file)
            send.putExtra(Intent.EXTRA_TEXT, clipText(text, MAX_TEXT_CHARS, note))
            if (file != null) attach(send, file, fileName, "text/plain")
        }
        start(context, Intent.createChooser(send, title))
    }

    /** The whole [text] as a file other apps may read; null when it cannot be written. */
    fun textFile(context: Context, text: String, fileName: String = SHARED_TEXT_FILE): Uri? {
        val file = File(Diagnostics.reportsDir(context), fileName)
        file.writeText(text, Charsets.UTF_8)
        return runCatching { FileProvider.getUriForFile(context, ArchiveFiles.authority(context), file) }.getOrNull()
    }

    /**
     * Hands the file at [uri] (a content address, or a path the app listed) to the app the person
     * picks. [type] defaults to what the provider or the file name says.
     */
    fun file(context: Context, uri: Uri, name: String, type: String? = null) {
        val target = shareable(context, uri)
        if (target == null) {
            Toast.makeText(context, R.string.share_failed, Toast.LENGTH_SHORT).show()
            return
        }
        val mime = type?.takeIf { it.isNotBlank() } ?: mimeOf(context, target, name)
        val send = Intent(Intent.ACTION_SEND).setType(mime)
        attach(send, target, name, mime)
        start(context, Intent.createChooser(send, name))
    }

    /**
     * A content address other apps can open: files the app listed by path go through its own
     * provider (a `file://` address would end the app on Android 7+). Null when that is not
     * possible (a path outside the provider's folders).
     */
    fun shareable(context: Context, uri: Uri): Uri? = when (uri.scheme) {
        "file" -> uri.path?.let { path ->
            runCatching { FileProvider.getUriForFile(context, ArchiveFiles.authority(context), File(path)) }.getOrNull()
        }
        else -> uri
    }

    /** The MIME type to share [uri] (named [name]) as; any type when nothing better is known. */
    private fun mimeOf(context: Context, uri: Uri, name: String): String {
        val fromProvider = runCatching { context.contentResolver.getType(uri) }.getOrNull()
        if (!fromProvider.isNullOrBlank()) return fromProvider
        return mimeFromName(name) ?: "*/*"
    }

    /** The MIME type a file name's extension stands for, if Android knows it. */
    fun mimeFromName(name: String): String? {
        val ext = name.substringAfterLast('.', "").lowercase(Locale.ROOT)
        if (ext.isEmpty()) return null
        return android.webkit.MimeTypeMap.getSingleton().getMimeTypeFromExtension(ext)
    }

    /**
     * [uri] as the shared stream, also in the intent's ClipData (where Android looks for the
     * addresses a read grant covers), with that grant.
     */
    private fun attach(send: Intent, uri: Uri, label: String, mime: String) {
        send.putExtra(Intent.EXTRA_STREAM, uri)
        send.clipData = ClipData(label, arrayOf(mime), ClipData.Item(uri))
        send.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
    }

    /** Starts [chooser]; tells the person instead of failing when nothing can show it. */
    fun start(context: Context, chooser: Intent) {
        if (context !is Activity) chooser.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        try {
            context.startActivity(chooser)
        } catch (_: ActivityNotFoundException) {
            Toast.makeText(context, R.string.share_no_app, Toast.LENGTH_SHORT).show()
        }
    }
}
