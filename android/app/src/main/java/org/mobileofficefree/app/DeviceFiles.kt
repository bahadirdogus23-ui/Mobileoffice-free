package org.mobileofficefree.app

import android.Manifest
import android.content.ContentUris
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import android.provider.Settings
import androidx.core.content.ContextCompat
import androidx.core.net.toUri

/** A document found on the device's shared storage. */
data class DeviceFile(val uri: Uri, val name: String, val kind: DocumentKind, val size: Long, val modifiedAt: Long)

/**
 * Lists every supported document on shared storage, newest first, through MediaStore.
 *
 * Seeing documents other apps made needs "All files access" on Android 11+ (Play allows it for
 * document managers, with a declaration) and READ_EXTERNAL_STORAGE before that. Without it the
 * app still opens anything handed to it; the home screen only shows the files it opened.
 */
object DeviceFiles {
    /** When the folders were last walked for unindexed documents (elapsed realtime). */
    @Volatile private var lastWalk = 0L

    fun hasAccess(context: Context): Boolean =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            Environment.isExternalStorageManager()
        } else {
            ContextCompat.checkSelfPermission(context, Manifest.permission.READ_EXTERNAL_STORAGE) == PackageManager.PERMISSION_GRANTED
        }

    /** Android 11+: the system screen where the person turns on "All files access" for this app. */
    fun accessSettingsIntent(context: Context): Intent =
        Intent(Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION, "package:${context.packageName}".toUri())

    fun fallbackSettingsIntent(): Intent = Intent(Settings.ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION)

    /**
     * How a found file is listed, or null to leave it out. XML files are listed only when they
     * are GİB e-invoices (their first KB say so); other XML belongs to apps, not people.
     */
    fun listedKind(context: Context, uri: Uri, name: String, mime: String?): DocumentKind? {
        if (name.endsWith(".xml", ignoreCase = true)) {
            val head = runCatching { context.contentResolver.openInputStream(uri)?.use { String(it.readNBytesUpTo(8 * 1024)) } }.getOrNull()
            return if (head != null && Invoices.looksLikeUbl(head)) DocumentKind.INVOICE else null
        }
        return DocumentKind.of(name, mime).takeIf { it != DocumentKind.OTHER }
    }

    /** Every supported document, newest first. Call off the main thread. */
    fun scan(context: Context, limit: Int = 5_000): List<DeviceFile> {
        val extensions = DocumentKind.listedExtensions.toList()
        // MediaStore keeps names as typed; LIKE is case-insensitive for ASCII, which file extensions are.
        // Folders are left out: a folder can be named like a document ("Dilekçe.udf" unpacked).
        val selection = "(" + extensions.joinToString(" OR ") { "${MediaStore.Files.FileColumns.DISPLAY_NAME} LIKE ?" } + ")" +
            " AND ($FORMAT_COLUMN IS NULL OR $FORMAT_COLUMN != $FORMAT_FOLDER)"
        val args = extensions.map { "%.$it" }.toTypedArray()
        val collection = MediaStore.Files.getContentUri("external")
        val projection = arrayOf(
            MediaStore.Files.FileColumns._ID,
            MediaStore.Files.FileColumns.DISPLAY_NAME,
            MediaStore.Files.FileColumns.SIZE,
            MediaStore.Files.FileColumns.DATE_MODIFIED,
            MediaStore.Files.FileColumns.MIME_TYPE,
            MediaStore.Files.FileColumns.DATA,
        )
        val out = ArrayList<DeviceFile>()
        val known = HashSet<String>()
        context.contentResolver.query(
            collection, projection, selection, args, "${MediaStore.Files.FileColumns.DATE_MODIFIED} DESC",
        )?.use { c ->
            while (c.moveToNext() && out.size < limit) {
                c.getString(5)?.let(known::add)
                val name = c.getString(1) ?: continue
                val uri = ContentUris.withAppendedId(collection, c.getLong(0))
                val kind = listedKind(context, uri, name, c.getString(4)) ?: continue
                out += DeviceFile(
                    uri = uri,
                    name = name,
                    kind = kind,
                    size = c.getLong(2),
                    modifiedAt = c.getLong(3) * 1000,
                )
            }
        }
        // MediaStore leaves out files it has not indexed (often UYAP's .udf and other types it
        // does not know): with access to every file, the folders are walked for them and the
        // system is asked to index what was missing, which gives each one its content address.
        // Files indexed this way stay in MediaStore, so the walk is needed only now and then.
        val now = android.os.SystemClock.elapsedRealtime()
        if (hasAccess(context) && out.size < limit && (lastWalk == 0L || now - lastWalk > WALK_EVERY_MS)) {
            lastWalk = now
            val root = Environment.getExternalStorageDirectory()
            // XML is left to the index: most of it belongs to apps, not people.
            val missing = findDocuments(root, extensions.map { it.lowercase(java.util.Locale.ROOT) }.toSet() - "xml") { it.path !in known }.take(MAX_INDEXED)
            out += indexed(context, missing.map { it.path }).mapNotNull { (file, uri) ->
                val kind = listedKind(context, uri, file.name, null) ?: return@mapNotNull null
                DeviceFile(uri = uri, name = file.name, kind = kind, size = file.length(), modifiedAt = file.lastModified())
            }
            out.sortByDescending { it.modifiedAt }
        }
        return out.take(limit)
    }

    /**
     * Files under [root] whose extension is one of [extensions] (lower case) and that [wanted]
     * accepts, newest first. Hidden folders and Android/ (apps' private data) are skipped; at most
     * [maxEntries] entries are looked at, so a huge card cannot stall the list.
     */
    fun findDocuments(root: java.io.File, extensions: Set<String>, maxEntries: Int = 200_000, wanted: (java.io.File) -> Boolean): List<java.io.File> {
        val found = ArrayList<java.io.File>()
        val stack = ArrayDeque<java.io.File>().apply { add(root) }
        var seen = 0
        while (stack.isNotEmpty() && seen < maxEntries) {
            val dir = stack.removeLast()
            val entries = dir.listFiles() ?: continue
            for (f in entries) {
                if (++seen > maxEntries) break
                if (f.name.startsWith(".")) continue
                if (f.isDirectory) {
                    if (!(dir == root && f.name.equals("Android", ignoreCase = true))) stack.add(f)
                } else if (f.name.substringAfterLast('.', "").lowercase(java.util.Locale.ROOT) in extensions && wanted(f)) {
                    found += f
                }
            }
        }
        return found.sortedByDescending { it.lastModified() }
    }

    /** Asks the system to index [paths] and waits a little for their content addresses. */
    private fun indexed(context: Context, paths: List<String>): List<Pair<java.io.File, Uri>> {
        if (paths.isEmpty()) return emptyList()
        val got = java.util.Collections.synchronizedList(ArrayList<Pair<java.io.File, Uri>>())
        val done = java.util.concurrent.CountDownLatch(paths.size)
        android.media.MediaScannerConnection.scanFile(context, paths.toTypedArray(), null) { path, uri ->
            if (uri != null) got += java.io.File(path) to uri
            done.countDown()
        }
        done.await(INDEX_WAIT_SECONDS, java.util.concurrent.TimeUnit.SECONDS)
        return got.toList()
    }
}

/** At most this many unindexed documents are added per scan; the next scan adds more. */
private const val MAX_INDEXED = 300
private const val INDEX_WAIT_SECONDS = 15L
private const val WALK_EVERY_MS = 10 * 60 * 1000L

/** MediaStore's `format` column (public in the provider, hidden in the SDK) and its value for a folder (MTP "association"). */
private const val FORMAT_COLUMN = "format"
private const val FORMAT_FOLDER = 0x3001
