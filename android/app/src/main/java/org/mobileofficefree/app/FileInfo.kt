package org.mobileofficefree.app

import android.content.ContentResolver
import android.net.Uri
import android.provider.OpenableColumns
import java.util.Locale
import java.util.zip.ZipInputStream

/**
 * A file handed to the app. [kind] comes from the name and MIME type, or, when they say nothing
 * useful (a WhatsApp attachment without an extension, "application/octet-stream"), from the
 * file's first bytes. [typedName] is [name] with the extension engines need to pick a reader.
 */
data class FileInfo(val name: String, val size: Long, val mime: String?, val kind: DocumentKind, val typedName: String) {

    companion object {
        fun of(resolver: ContentResolver, uri: Uri): FileInfo {
            var name: String? = null
            var size = -1L
            resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE), null, null, null)?.use { c ->
                if (c.moveToFirst()) {
                    name = c.getString(0)
                    if (!c.isNull(1)) size = c.getLong(1)
                }
            }
            val shown = name ?: uri.lastPathSegment?.substringAfterLast('/') ?: "document"
            val mime = resolver.getType(uri)
            val byName = DocumentKind.of(shown, mime)
            val ext = shown.substringAfterLast('.', "").lowercase(Locale.ROOT)
            if (byName != DocumentKind.OTHER && ext in byName.extensions) return FileInfo(shown, size, mime, byName, shown)
            // The name has no usable extension: look inside.
            val sniffed = runCatching { resolver.openInputStream(uri)?.use(::sniff) }.getOrNull()
            return when {
                sniffed != null -> FileInfo(shown, size, mime, DocumentKind.of("x.$sniffed", null), "$shown.$sniffed")
                byName != DocumentKind.OTHER -> FileInfo(shown, size, mime, byName, shown)
                else -> FileInfo(shown, size, mime, DocumentKind.OTHER, shown)
            }
        }

        /** The extension a file's content says it has, from its signature, or null. */
        internal fun sniff(input: java.io.InputStream): String? {
            val buffered = input.buffered()
            buffered.mark(16)
            val head = ByteArray(12)
            val n = buffered.read(head)
            if (n < 4) return null
            fun at(i: Int, vararg b: Int) = b.withIndex().all { (k, v) -> i + k < n && (head[i + k].toInt() and 0xFF) == v }
            return when {
                at(0, 0x25, 0x50, 0x44, 0x46) -> "pdf" // %PDF
                at(0, 0x89, 0x50, 0x4E, 0x47) -> "png"
                at(0, 0xFF, 0xD8, 0xFF) -> "jpg"
                at(0, 0x47, 0x49, 0x46, 0x38) -> "gif"
                at(0, 0x52, 0x49, 0x46, 0x46) && at(8, 0x57, 0x45, 0x42, 0x50) -> "webp"
                at(0, 0x42, 0x4D) -> "bmp"
                at(0, 0x41, 0x43, 0x31, 0x30) -> "dwg" // AC10xx
                at(0, 0x7B, 0x5C, 0x72, 0x74, 0x66) -> "rtf" // {\\rtf
                at(0, 0xD0, 0xCF, 0x11, 0xE0) -> null // Legacy Office: Word, Excel and PowerPoint share this box.
                at(0, 0x50, 0x4B, 0x03, 0x04) -> {
                    buffered.reset()
                    zipKind(buffered)
                }
                else -> null
            }
        }

        /**
         * Office Open XML and OpenDocument files are ZIPs; their first entries tell which. A ZIP
         * with content.xml but no OpenDocument "mimetype" is a UYAP UDF document.
         */
        private fun zipKind(input: java.io.InputStream): String? {
            var content = false
            ZipInputStream(input).use { zip ->
                repeat(64) {
                    val e = zip.nextEntry ?: return if (content) "udf" else "zip"
                    when {
                        e.name == "mimetype" -> return when (zip.readBytes().decodeToString().trim()) {
                            "application/vnd.oasis.opendocument.text" -> "odt"
                            "application/vnd.oasis.opendocument.spreadsheet" -> "ods"
                            "application/vnd.oasis.opendocument.presentation" -> "odp"
                            else -> null
                        }
                        e.name.startsWith("word/") -> return "docx"
                        e.name.startsWith("xl/") -> return "xlsx"
                        e.name.startsWith("ppt/") -> return "pptx"
                        e.name == "content.xml" -> content = true
                    }
                }
            }
            // A ZIP of something else: show what is inside.
            return if (content) "udf" else "zip"
        }
    }
}
