package org.mobileofficefree.app

import android.content.ContentResolver
import android.net.Uri
import android.provider.OpenableColumns

data class FileInfo(val name: String, val size: Long, val mime: String?) {
    val kind: DocumentKind get() = DocumentKind.of(name, mime)

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
            val fallback = uri.lastPathSegment?.substringAfterLast('/') ?: "document"
            return FileInfo(name ?: fallback, size, resolver.getType(uri))
        }
    }
}
