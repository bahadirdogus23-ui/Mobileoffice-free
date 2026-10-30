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
        val selection = extensions.joinToString(" OR ") { "${MediaStore.Files.FileColumns.DISPLAY_NAME} LIKE ?" }
        val args = extensions.map { "%.$it" }.toTypedArray()
        val collection = MediaStore.Files.getContentUri("external")
        val projection = arrayOf(
            MediaStore.Files.FileColumns._ID,
            MediaStore.Files.FileColumns.DISPLAY_NAME,
            MediaStore.Files.FileColumns.SIZE,
            MediaStore.Files.FileColumns.DATE_MODIFIED,
            MediaStore.Files.FileColumns.MIME_TYPE,
        )
        val out = ArrayList<DeviceFile>()
        context.contentResolver.query(
            collection, projection, selection, args, "${MediaStore.Files.FileColumns.DATE_MODIFIED} DESC",
        )?.use { c ->
            while (c.moveToNext() && out.size < limit) {
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
        return out
    }
}
