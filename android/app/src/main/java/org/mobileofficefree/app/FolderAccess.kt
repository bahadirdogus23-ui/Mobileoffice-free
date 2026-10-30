package org.mobileofficefree.app

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.DocumentsContract
import androidx.core.content.edit

/**
 * Folders the person picked with the system folder picker, for those who would rather not give
 * "All files access": the app keeps read access to just these and lists the documents in them.
 */
class FolderAccess(context: Context) {
    private val app = context.applicationContext
    private val prefs = app.getSharedPreferences("folders", Context.MODE_PRIVATE)

    fun all(): List<Uri> = prefs.getStringSet(KEY, emptySet()).orEmpty().sorted().map(Uri::parse)

    fun add(tree: Uri) {
        runCatching { app.contentResolver.takePersistableUriPermission(tree, Intent.FLAG_GRANT_READ_URI_PERMISSION) }
        prefs.edit { putStringSet(KEY, all().map(Uri::toString).toSet() + tree.toString()) }
    }

    fun remove(tree: Uri) {
        runCatching { app.contentResolver.releasePersistableUriPermission(tree, Intent.FLAG_GRANT_READ_URI_PERMISSION) }
        prefs.edit { putStringSet(KEY, all().map(Uri::toString).toSet() - tree.toString()) }
    }

    /** A folder's name as the person knows it ("Download", "Belgeler"). */
    fun label(tree: Uri): String =
        DocumentsContract.getTreeDocumentId(tree).substringAfterLast(':').substringAfterLast('/').ifEmpty { tree.lastPathSegment.orEmpty() }

    /** Supported documents in the picked folders and their subfolders, newest first. Call off the main thread. */
    fun scan(limit: Int = 5_000): List<DeviceFile> {
        val out = ArrayList<DeviceFile>()
        for (tree in all()) {
            // Breadth first, a few levels deep: enough for Documents/Work/2026, not a crawl of the whole card.
            var level = listOf(DocumentsContract.getTreeDocumentId(tree))
            repeat(MAX_DEPTH) {
                val next = ArrayList<String>()
                for (parent in level) {
                    if (out.size >= limit) return@repeat
                    children(tree, parent, out, next)
                }
                level = next
            }
        }
        return out.sortedByDescending { it.modifiedAt }
    }

    private fun children(tree: Uri, parent: String, files: MutableList<DeviceFile>, folders: MutableList<String>) {
        val uri = DocumentsContract.buildChildDocumentsUriUsingTree(tree, parent)
        val projection = arrayOf(
            DocumentsContract.Document.COLUMN_DOCUMENT_ID,
            DocumentsContract.Document.COLUMN_DISPLAY_NAME,
            DocumentsContract.Document.COLUMN_MIME_TYPE,
            DocumentsContract.Document.COLUMN_SIZE,
            DocumentsContract.Document.COLUMN_LAST_MODIFIED,
        )
        runCatching {
            app.contentResolver.query(uri, projection, null, null, null)?.use { c ->
                while (c.moveToNext()) {
                    val id = c.getString(0) ?: continue
                    val name = c.getString(1) ?: continue
                    val mime = c.getString(2)
                    if (mime == DocumentsContract.Document.MIME_TYPE_DIR) {
                        if (!name.startsWith('.')) folders += id
                        continue
                    }
                    val ext = name.substringAfterLast('.', "").lowercase(java.util.Locale.ROOT)
                    if (ext !in DocumentKind.listedExtensions) continue
                    val uri = DocumentsContract.buildDocumentUriUsingTree(tree, id)
                    val kind = DeviceFiles.listedKind(app, uri, name, mime) ?: continue
                    files += DeviceFile(
                        uri = uri,
                        name = name,
                        kind = kind,
                        size = if (c.isNull(3)) -1 else c.getLong(3),
                        modifiedAt = if (c.isNull(4)) 0 else c.getLong(4),
                    )
                }
            }
        }
    }

    private companion object {
        const val KEY = "trees"
        const val MAX_DEPTH = 6
    }
}
