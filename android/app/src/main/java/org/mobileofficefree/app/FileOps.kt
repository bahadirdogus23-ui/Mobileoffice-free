package org.mobileofficefree.app

import android.app.RecoverableSecurityException
import android.content.ContentUris
import android.content.ContentValues
import android.content.Context
import android.content.IntentSender
import android.media.MediaScannerConnection
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Environment
import android.provider.DocumentsContract
import android.provider.MediaStore
import android.provider.OpenableColumns
import android.util.Log
import androidx.core.content.FileProvider
import java.io.File
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

/** The logcat tag for the home list's file actions (rename, copy, trash, delete). */
internal const val FILES_LOG_TAG = "BadoFiles"

/** What a file action came to. */
sealed interface FileOutcome {
    /** Done: the file is now at [uri] (the same address, or a new one after a rename or a copy), named [name]. */
    data class Done(val uri: Uri, val name: String) : FileOutcome

    /**
     * The system must ask the person first (Android's write, trash or delete confirmation). After
     * they allow it, [granted] is what happened; null means the action is to be run again.
     */
    class NeedsConsent(val sender: IntentSender, val granted: Done?) : FileOutcome

    /** The copy cannot go next to the file: the person picks a place for [name] (of type [mime]). */
    class NeedsLocation(val name: String, val mime: String) : FileOutcome

    /** The person said no to the system's question. */
    data object Cancelled : FileOutcome

    /** Could not be done; [message] says why, in the person's language. */
    data class Failed(val message: String) : FileOutcome
}

/** How a file can be deleted: to the system's trash (recoverable), for good, or not from here. */
enum class DeleteWay { TRASH, PERMANENT, NONE }

/** What the "Details" dialog shows; null fields are not known (or too slow to learn). */
data class FileDetails(val name: String, val mime: String?, val size: Long, val folder: String?, val modified: Long?)

/**
 * Rename, copy and delete for the home list's files, each in the way the file's address allows:
 * MediaStore (the device list; the person's consent through the system where Android asks for it),
 * the file itself (with "All files access"), or the Storage Access Framework (picked folders and
 * files). Never overwrites a file, and deletion goes to the system's trash where there is one.
 * Every function does I/O: call off the main thread.
 */
object FileOps {
    private fun log(message: String) {
        Log.i(FILES_LOG_TAG, message)
    }

    private fun warn(message: String, e: Throwable? = null) {
        Log.w(FILES_LOG_TAG, message, e)
    }

    private fun isMedia(uri: Uri) = uri.scheme == "content" && uri.authority == MediaStore.AUTHORITY

    private fun isDocument(context: Context, uri: Uri) = runCatching { DocumentsContract.isDocumentUri(context, uri) }.getOrDefault(false)

    /** Android 11+ with "All files access": the app may rename, copy and delete files directly. */
    fun canWriteFiles(): Boolean = Build.VERSION.SDK_INT >= Build.VERSION_CODES.R && Environment.isExternalStorageManager()

    /** The file on disk behind [uri], when the app can know it (a listed path, MediaStore's record, its own provider). */
    fun pathOf(context: Context, uri: Uri): File? = runCatching {
        when {
            uri.scheme == "file" -> uri.path?.let(::File)
            isMedia(uri) -> context.contentResolver.query(uri, arrayOf(MediaStore.MediaColumns.DATA), null, null, null)?.use { c ->
                if (c.moveToFirst()) c.getString(0)?.let(::File) else null
            }
            // The app's own provider: "storage" is shared storage's root (res/xml/file_paths.xml).
            uri.authority == ArchiveFiles.authority(context) -> uri.pathSegments.takeIf { it.size >= 2 && it[0] == "storage" }
                ?.let { File(Environment.getExternalStorageDirectory(), it.drop(1).joinToString("/")) }
            else -> null
        }
    }.getOrNull()

    /** MediaStore's address for [file], if it is indexed. */
    private fun mediaUriFor(context: Context, file: File): Uri? = runCatching {
        val files = MediaStore.Files.getContentUri("external")
        context.contentResolver.query(files, arrayOf(MediaStore.MediaColumns._ID), "${MediaStore.MediaColumns.DATA} = ?", arrayOf(file.path), null)?.use { c ->
            if (c.moveToFirst()) ContentUris.withAppendedId(files, c.getLong(0)) else null
        }
    }.getOrNull()

    /** [uri] as MediaStore knows it: itself, or the indexed record of the file behind it. */
    private fun mediaFor(context: Context, uri: Uri): Uri? = when {
        isMedia(uri) -> uri
        // A file picked through the system's Files app: MediaStore's record of the same file, so it can go to the trash.
        isDocument(context, uri) && Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q -> runCatching { MediaStore.getMediaUri(context, uri) }.getOrNull()
        else -> pathOf(context, uri)?.takeIf { it.isFile }?.let { mediaUriFor(context, it) }
    }

    /** Asks MediaStore to index [paths] again (a removed path drops out) and waits a little for the new addresses. */
    private fun scan(context: Context, paths: List<String>): Map<String, Uri> {
        if (paths.isEmpty()) return emptyMap()
        val got = ConcurrentHashMap<String, Uri>()
        val done = CountDownLatch(paths.size)
        MediaScannerConnection.scanFile(context, paths.toTypedArray(), null) { path, uri ->
            if (uri != null) got[path] = uri
            done.countDown()
        }
        done.await(SCAN_WAIT_SECONDS, TimeUnit.SECONDS)
        return got
    }

    /** The name the provider gives [uri] now, if it answers. */
    private fun displayName(context: Context, uri: Uri): String? = runCatching {
        context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { c -> if (c.moveToFirst()) c.getString(0) else null }
    }.getOrNull()

    /** MediaStore's folder of [uri] ("Documents/İş/"), Android 10+. */
    private fun relativePath(context: Context, uri: Uri): String? {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.Q) return null
        return runCatching {
            context.contentResolver.query(uri, arrayOf(MediaStore.MediaColumns.RELATIVE_PATH), null, null, null)?.use { c -> if (c.moveToFirst()) c.getString(0) else null }
        }.getOrNull()
    }

    /** Whether MediaStore has a file named [name] in [relative]. */
    private fun mediaNameTaken(context: Context, relative: String, name: String): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.Q) return false
        return runCatching {
            context.contentResolver.query(
                MediaStore.Files.getContentUri("external"),
                arrayOf(MediaStore.MediaColumns._ID),
                "${MediaStore.MediaColumns.RELATIVE_PATH} = ? AND ${MediaStore.MediaColumns.DISPLAY_NAME} = ?",
                arrayOf(relative, name),
                null,
            )?.use { it.count > 0 } ?: false
        }.getOrDefault(false)
    }

    /** For a document in a picked folder: the folder's address and the names in it. */
    private fun treeParent(context: Context, uri: Uri): Pair<Uri, Set<String>>? = runCatching {
        if (!DocumentsContract.isTreeUri(uri)) return null
        val path = DocumentsContract.findDocumentPath(context.contentResolver, uri)?.path ?: return null
        val parentId = path.getOrNull(path.size - 2) ?: return null
        val names = HashSet<String>()
        context.contentResolver.query(
            DocumentsContract.buildChildDocumentsUriUsingTree(uri, parentId),
            arrayOf(DocumentsContract.Document.COLUMN_DISPLAY_NAME), null, null, null,
        )?.use { c -> while (c.moveToNext()) c.getString(0)?.let(names::add) }
        DocumentsContract.buildDocumentUriUsingTree(uri, parentId) to names
    }.getOrNull()

    /** Whether the folder of [uri] already has a file named [name] (other than the file itself, letter case aside). */
    fun nameTaken(context: Context, uri: Uri, oldName: String, name: String): Boolean {
        if (FileNames.sameIgnoringCase(oldName, name)) return false
        pathOf(context, uri)?.parentFile?.let { return File(it, name).exists() }
        if (isMedia(uri)) relativePath(context, uri)?.let { return mediaNameTaken(context, it, name) }
        if (isDocument(context, uri)) treeParent(context, uri)?.let { return name in it.second }
        return false
    }

    /** The type to give a copy of [uri] named [name]. */
    fun mimeOf(context: Context, uri: Uri, name: String): String =
        runCatching { context.contentResolver.getType(uri) }.getOrNull()?.takeIf { it.isNotBlank() && it != "application/octet-stream" }
            ?: Share.mimeFromName(name) ?: "application/octet-stream"

    /** The system's confirmation for writing to [uri] after [e], if Android has one for it. */
    private fun writeConsent(context: Context, uri: Uri, e: SecurityException): IntentSender? = when {
        Build.VERSION.SDK_INT >= Build.VERSION_CODES.R -> runCatching { MediaStore.createWriteRequest(context.contentResolver, listOf(uri)).intentSender }.getOrNull()
        Build.VERSION.SDK_INT == Build.VERSION_CODES.Q && e is RecoverableSecurityException -> e.userAction.actionIntent.intentSender
        else -> null
    }

    // --- Rename ----------------------------------------------------------------------------

    /**
     * Renames the file at [uri] (named [oldName]) to [newName]. [consented]: the person already
     * allowed it in the system's dialog (asked once, not again).
     */
    fun rename(context: Context, uri: Uri, oldName: String, newName: String, consented: Boolean = false): FileOutcome {
        val name = newName.trim()
        if (name == oldName) return FileOutcome.Done(uri, name)
        if (nameTaken(context, uri, oldName, name)) {
            log("rename: $uri \"$oldName\" -> \"$name\": the name is taken")
            return FileOutcome.Failed(context.getString(R.string.rename_error_exists))
        }
        val file = pathOf(context, uri)?.takeIf { it.isFile }
        val target = file?.parentFile?.let { File(it, name) }
        log("rename: $uri \"$oldName\" -> \"$name\" (file ${file?.path}, all files access ${canWriteFiles()}, consented $consented)")

        if (isMedia(uri) && Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            // MediaStore renames the file itself and keeps its address (recent files keep working).
            try {
                val changed = context.contentResolver.update(uri, ContentValues().apply { put(MediaStore.MediaColumns.DISPLAY_NAME, name) }, null, null)
                log("rename: MediaStore update changed $changed row(s); new file there: ${target?.exists()}")
                if (changed > 0 && (target == null || target.exists())) return FileOutcome.Done(uri, displayName(context, uri) ?: name)
            } catch (e: SecurityException) {
                warn("rename: MediaStore refused the update", e)
                if (!canWriteFiles()) {
                    if (consented) return FileOutcome.Failed(context.getString(R.string.file_no_permission))
                    val ask = writeConsent(context, uri, e) ?: return FileOutcome.Failed(context.getString(R.string.file_no_permission))
                    return FileOutcome.NeedsConsent(ask, granted = null)
                }
            } catch (e: Exception) {
                // Tried below on the file itself where the app may.
                warn("rename: MediaStore update failed", e)
            }
        }

        if (file != null && target != null && canWriteFiles()) {
            if (!file.renameTo(target)) {
                log("rename: File.renameTo(${target.path}) failed")
                return FileOutcome.Failed(context.getString(R.string.file_action_failed))
            }
            // MediaStore must follow (the old path dropped, the new one indexed), so that later
            // actions (delete, trash, details) find the file under its new name.
            val indexed = scan(context, listOf(file.path, target.path))[target.path]
            val newUri = when {
                isMedia(uri) -> indexed ?: mediaUriFor(context, target) ?: Uri.fromFile(target)
                uri.scheme == "file" -> indexed ?: Uri.fromFile(target).also { DeviceFiles.pathRenamed(file, target) }
                else -> runCatching { FileProvider.getUriForFile(context, ArchiveFiles.authority(context), target) }.getOrDefault(Uri.fromFile(target))
            }
            log("rename: renamed on disk to ${target.path}; indexed as $indexed; listed as $newUri")
            return FileOutcome.Done(newUri, name)
        }

        if (isDocument(context, uri)) {
            return try {
                val renamed = DocumentsContract.renameDocument(context.contentResolver, uri, name) ?: uri
                log("rename: document renamed, now $renamed")
                // A provider may change the name a little to keep it unique ("ad (1).pdf").
                FileOutcome.Done(renamed, displayName(context, renamed) ?: name)
            } catch (e: Exception) {
                warn("rename: document rename failed", e)
                FileOutcome.Failed(context.getString(R.string.file_no_permission))
            }
        }
        log("rename: no way to rename $uri from here")
        return FileOutcome.Failed(context.getString(if (isMedia(uri) || file != null) R.string.file_no_permission else R.string.file_not_here))
    }

    // --- Copy ------------------------------------------------------------------------------

    private fun copyBytes(context: Context, from: Uri, to: Uri) {
        require(from != to) { "same file" }
        val input = context.contentResolver.openInputStream(from) ?: error("no data")
        input.use { i ->
            val output = context.contentResolver.openOutputStream(to, "wt") ?: error("cannot write")
            output.use { i.copyTo(it) }
        }
    }

    /** Copies the file at [uri] (named [name]) to "<name> (copy word).ext" next to it. */
    fun duplicate(context: Context, uri: Uri, name: String, copyWord: String): FileOutcome {
        val mime = mimeOf(context, uri, name)
        val file = pathOf(context, uri)?.takeIf { it.isFile }
        if (file != null && canWriteFiles()) {
            val dir = file.parentFile ?: return FileOutcome.Failed(context.getString(R.string.file_action_failed))
            val copyName = FileNames.uniqueCopyName(file.name, copyWord) { File(dir, it).exists() }
            val target = File(dir, copyName)
            return try {
                // createNewFile: never over a file that appeared meanwhile.
                if (!target.createNewFile()) return FileOutcome.Failed(context.getString(R.string.rename_error_exists))
                file.inputStream().use { input -> target.outputStream().use { input.copyTo(it) } }
                val indexed = scan(context, listOf(target.path))[target.path]
                if (indexed == null) DeviceFiles.pathAdded(target)
                FileOutcome.Done(indexed ?: Uri.fromFile(target), copyName)
            } catch (e: Exception) {
                target.delete()
                FileOutcome.Failed(context.getString(R.string.file_action_failed_because, e.message ?: e.javaClass.simpleName))
            }
        }

        if (isMedia(uri) && Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            val relative = relativePath(context, uri)
            if (relative != null) {
                val copyName = FileNames.uniqueCopyName(name, copyWord) { mediaNameTaken(context, relative, it) }
                val values = ContentValues().apply {
                    put(MediaStore.MediaColumns.DISPLAY_NAME, copyName)
                    put(MediaStore.MediaColumns.MIME_TYPE, mime)
                    put(MediaStore.MediaColumns.RELATIVE_PATH, relative)
                    put(MediaStore.MediaColumns.IS_PENDING, 1)
                }
                // Android lets apps add documents only to some folders (Documents, Download…): elsewhere the person picks a place.
                val dest = runCatching { context.contentResolver.insert(MediaStore.Files.getContentUri("external"), values) }.getOrNull()
                if (dest != null) {
                    try {
                        copyBytes(context, uri, dest)
                        context.contentResolver.update(dest, ContentValues().apply { put(MediaStore.MediaColumns.IS_PENDING, 0) }, null, null)
                        return FileOutcome.Done(dest, displayName(context, dest) ?: copyName)
                    } catch (_: Exception) {
                        // The record the app just made (never the original).
                        runCatching { context.contentResolver.delete(dest, null, null) }
                    }
                }
            }
        }

        if (isDocument(context, uri)) {
            val parent = treeParent(context, uri)
            if (parent != null) {
                val copyName = FileNames.uniqueCopyName(name, copyWord) { it in parent.second }
                val dest = runCatching { DocumentsContract.createDocument(context.contentResolver, parent.first, mime, copyName) }.getOrNull()
                if (dest != null) {
                    try {
                        copyBytes(context, uri, dest)
                        return FileOutcome.Done(dest, displayName(context, dest) ?: copyName)
                    } catch (_: Exception) {
                        runCatching { DocumentsContract.deleteDocument(context.contentResolver, dest) }
                    }
                }
            }
        }
        return FileOutcome.NeedsLocation(FileNames.copyName(name, copyWord), mime)
    }

    /** Writes a copy of [from] to [to], a new file the person made in the system's "Save" dialog. */
    fun copyTo(context: Context, from: Uri, to: Uri, name: String): FileOutcome = try {
        copyBytes(context, from, to)
        FileOutcome.Done(to, displayName(context, to) ?: name)
    } catch (e: Exception) {
        FileOutcome.Failed(context.getString(R.string.file_action_failed_because, e.message ?: e.javaClass.simpleName))
    }

    // --- Delete ----------------------------------------------------------------------------

    /** How the file at [uri] would be deleted, asked before the confirmation so it can say so. */
    fun deleteWay(context: Context, uri: Uri): DeleteWay = when {
        Build.VERSION.SDK_INT >= Build.VERSION_CODES.R && mediaFor(context, uri) != null -> DeleteWay.TRASH
        pathOf(context, uri)?.isFile == true && canWriteFiles() -> DeleteWay.PERMANENT
        isMedia(uri) && Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q -> DeleteWay.PERMANENT
        isDocument(context, uri) -> DeleteWay.PERMANENT
        else -> DeleteWay.NONE
    }.also { log("delete way for $uri: $it") }

    /**
     * Moves the file at [uri] to the system's trash without asking Android first, which an app with
     * "All files access" may do on Android 11+ (MediaStore renames it to ".trashed-<expiry>-<name>"
     * and removes it after about 30 days). Only after the person confirmed in the app's own dialog.
     * False when it could not (the caller then asks Android through [trash]); the file is then left
     * as it was.
     */
    fun trashDirectly(context: Context, uri: Uri): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R) return false
        if (!canWriteFiles()) {
            log("trash directly: no all files access for $uri")
            return false
        }
        val media = mediaFor(context, uri)
        if (media == null) {
            log("trash directly: MediaStore has no record of $uri")
            return false
        }
        val before = pathOf(context, media)?.takeIf { it.isFile }
        return try {
            val changed = context.contentResolver.update(media, ContentValues().apply { put(MediaStore.MediaColumns.IS_TRASHED, 1) }, null, null)
            val stillThere = before?.exists() == true
            log("trash directly: $uri (record $media, file ${before?.path}): $changed row(s) changed, file still at its path: $stillThere")
            if (changed > 0 && !stillThere) return true
            if (changed > 0) {
                // Marked as trashed but not moved: undone, so the record matches the file again.
                val extras = Bundle().apply { putInt(MediaStore.QUERY_ARG_MATCH_TRASHED, MediaStore.MATCH_INCLUDE) }
                val undo = ContentValues().apply { put(MediaStore.MediaColumns.IS_TRASHED, 0) }
                val undone = runCatching { context.contentResolver.update(media, undo, extras) }.getOrDefault(0)
                log("trash directly: the file did not move; trashed flag undone on $undone row(s)")
            }
            false
        } catch (e: Exception) {
            warn("trash directly: MediaStore refused to trash $media", e)
            false
        }
    }

    /**
     * The system's request to move [uris] to its trash (Android 11+; kept about 30 days, then
     * removed). Failed when Android will not trash them (the caller then asks about deleting for
     * good): Android makes such requests only for photos, videos and music, and throws for
     * documents addressed through MediaStore's "files" table, as the home list's are.
     */
    fun trash(context: Context, uris: List<Uri>): FileOutcome {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R) return FileOutcome.Failed(context.getString(R.string.delete_trash_unavailable))
        val media = uris.mapNotNull { mediaFor(context, it) }
        log("trash request: ${uris.size} file(s), MediaStore records: ${media.joinToString()}")
        if (media.size != uris.size || media.isEmpty()) return FileOutcome.Failed(context.getString(R.string.delete_trash_unavailable))
        return try {
            FileOutcome.NeedsConsent(MediaStore.createTrashRequest(context.contentResolver, media, true).intentSender, FileOutcome.Done(uris.first(), ""))
        } catch (e: Exception) {
            warn("trash request: Android would not make one", e)
            FileOutcome.Failed(context.getString(R.string.delete_trash_unavailable))
        }
    }

    /** Deletes the file at [uri] for good. Only after the person confirmed that it cannot be undone. */
    fun deletePermanently(context: Context, uri: Uri, consented: Boolean = false): FileOutcome {
        val file = pathOf(context, uri)?.takeIf { it.isFile }
        log("delete for good: $uri (file ${file?.path}, all files access ${canWriteFiles()}, consented $consented)")
        if (file != null && canWriteFiles()) {
            if (!file.delete()) {
                log("delete for good: File.delete failed for ${file.path}")
                return FileOutcome.Failed(context.getString(R.string.file_action_failed))
            }
            scan(context, listOf(file.path))
            DeviceFiles.pathRemoved(file)
            log("delete for good: ${file.path} deleted")
            return FileOutcome.Done(uri, file.name)
        }
        if (isMedia(uri) && Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            return try {
                FileOutcome.NeedsConsent(MediaStore.createDeleteRequest(context.contentResolver, listOf(uri)).intentSender, FileOutcome.Done(uri, ""))
            } catch (e: Exception) {
                warn("delete for good: Android would not make a delete request for $uri", e)
                FileOutcome.Failed(context.getString(R.string.file_no_permission))
            }
        }
        if (isMedia(uri) && Build.VERSION.SDK_INT == Build.VERSION_CODES.Q) {
            return try {
                if (context.contentResolver.delete(uri, null, null) > 0) FileOutcome.Done(uri, "") else FileOutcome.Failed(context.getString(R.string.file_action_failed))
            } catch (e: RecoverableSecurityException) {
                if (consented) FileOutcome.Failed(context.getString(R.string.file_no_permission)) else FileOutcome.NeedsConsent(e.userAction.actionIntent.intentSender, granted = null)
            } catch (_: Exception) {
                FileOutcome.Failed(context.getString(R.string.file_no_permission))
            }
        }
        if (isDocument(context, uri)) {
            val deleted = runCatching { DocumentsContract.deleteDocument(context.contentResolver, uri) }.getOrDefault(false)
            return if (deleted) FileOutcome.Done(uri, "") else FileOutcome.Failed(context.getString(R.string.file_no_permission))
        }
        return FileOutcome.Failed(context.getString(R.string.file_not_here))
    }

    // --- Details ---------------------------------------------------------------------------

    /** Name, type, size, folder and modification time, from what the provider answers (no engine, no reading the file). */
    fun details(context: Context, uri: Uri, name: String, size: Long, date: Long): FileDetails {
        val file = pathOf(context, uri)?.takeIf { it.isFile }
        var shownName = name
        var shownSize = size
        runCatching {
            context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE), null, null, null)?.use { c ->
                if (c.moveToFirst()) {
                    c.getString(0)?.let { shownName = it }
                    if (!c.isNull(1)) shownSize = c.getLong(1)
                }
            }
        }
        if (file != null) shownSize = file.length()
        val modified = when {
            file != null -> file.lastModified().takeIf { it > 0 }
            isMedia(uri) -> runCatching {
                context.contentResolver.query(uri, arrayOf(MediaStore.MediaColumns.DATE_MODIFIED), null, null, null)?.use { c ->
                    if (c.moveToFirst() && !c.isNull(0)) c.getLong(0) * 1000 else null
                }
            }.getOrNull()
            isDocument(context, uri) -> runCatching {
                context.contentResolver.query(uri, arrayOf(DocumentsContract.Document.COLUMN_LAST_MODIFIED), null, null, null)?.use { c ->
                    if (c.moveToFirst() && !c.isNull(0)) c.getLong(0) else null
                }
            }.getOrNull()
            else -> null
        } ?: date.takeIf { it > 0 }
        val folder = file?.parent ?: (if (isMedia(uri)) relativePath(context, uri) else null)
            ?: if (isDocument(context, uri)) runCatching { DocumentsContract.getDocumentId(uri).substringAfter(':', "").substringBeforeLast('/', "").ifEmpty { null } }.getOrNull() else null
        val mime = runCatching { context.contentResolver.getType(uri) }.getOrNull() ?: Share.mimeFromName(shownName)
        return FileDetails(shownName, mime, shownSize, folder, modified)
    }

    private const val SCAN_WAIT_SECONDS = 10L
}
