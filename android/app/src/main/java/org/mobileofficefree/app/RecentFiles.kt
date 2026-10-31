package org.mobileofficefree.app

import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.core.content.edit
import org.json.JSONArray
import org.json.JSONObject

data class RecentFile(
    val uri: Uri,
    val name: String,
    val kind: DocumentKind,
    val size: Long,
    val openedAt: Long,
    /** How far the person read, 0–100 (shown as "Görüntülenen %N"). */
    val progress: Int = 0,
)

/**
 * Files opened in the app, newest first. Only what the person chose to open is listed: the app has
 * no storage permission and keeps a persistable grant for each entry where the provider allows one.
 */
class RecentFiles(context: Context) {
    private val app = context.applicationContext
    private val prefs = app.getSharedPreferences("recent_files", Context.MODE_PRIVATE)

    fun all(): List<RecentFile> {
        val arr = runCatching { JSONArray(prefs.getString(KEY, "[]")) }.getOrDefault(JSONArray())
        return (0 until arr.length()).mapNotNull { i ->
            val o = arr.optJSONObject(i) ?: return@mapNotNull null
            runCatching {
                RecentFile(
                    uri = Uri.parse(o.getString("uri")),
                    name = o.getString("name"),
                    kind = DocumentKind.valueOf(o.getString("kind")),
                    size = o.optLong("size", -1),
                    openedAt = o.getLong("openedAt"),
                    progress = o.optInt("progress", 0),
                )
            }.getOrNull()
        }
    }

    fun remember(uri: Uri, name: String, kind: DocumentKind, size: Long) {
        // Files unpacked from an archive live in the cache only while it is open.
        if (uri.authority == ArchiveFiles.authority(app)) return
        // Incognito: nothing is listed (the file opens all the same).
        if (PrivacySettings.get(app).incognito) return
        runCatching {
            app.contentResolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        val previous = all().firstOrNull { it.uri == uri }
        save(listOf(RecentFile(uri, name, kind, size, System.currentTimeMillis(), previous?.progress ?: 0)) +
            all().filter { it.uri != uri })
    }

    fun setProgress(uri: Uri, progress: Int) {
        save(all().map { if (it.uri == uri) it.copy(progress = progress.coerceIn(0, 100)) else it })
    }

    /** Empties the list and gives back the access kept for each file. */
    fun clear() {
        all().forEach { forget(it.uri) }
    }

    fun forget(uri: Uri) {
        runCatching {
            app.contentResolver.releasePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        save(all().filter { it.uri != uri })
    }

    private fun save(files: List<RecentFile>) {
        val arr = JSONArray()
        files.take(MAX).forEach {
            arr.put(
                JSONObject()
                    .put("uri", it.uri.toString())
                    .put("name", it.name)
                    .put("kind", it.kind.name)
                    .put("size", it.size)
                    .put("openedAt", it.openedAt)
                    .put("progress", it.progress),
            )
        }
        prefs.edit { putString(KEY, arr.toString()) }
    }

    private companion object {
        const val KEY = "files"
        const val MAX = 200
    }
}
