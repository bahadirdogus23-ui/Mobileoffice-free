package org.mobileofficefree.app

import android.content.Context
import android.net.Uri
import java.io.File
import java.security.MessageDigest

/**
 * Auto-recovery, as Office keeps it: unsaved edits are copied every so often into the app's own
 * private folder (never over the person's file), so a crash or a killed app loses at most a few
 * seconds of work; opening the file again offers the copy back.
 */
class RecoveryStore(context: Context) {
    private val dir = File(context.filesDir, "recovery").apply { mkdirs() }

    /** A kept copy: its document name (the format to open it as), bytes on disk and when it was made. */
    class Copy(val name: String, val file: File, val savedAt: Long)

    private fun key(uri: Uri): String =
        MessageDigest.getInstance("SHA-1").digest(uri.toString().toByteArray()).joinToString("") { "%02x".format(it) }

    /** Keeps [bytes] (a document named [name]) as the recovery copy of the file at [uri]. */
    fun write(uri: Uri, name: String, bytes: ByteArray) {
        val k = key(uri)
        // Written beside, then moved over: a copy is never left half written.
        val tmp = File(dir, "$k.tmp")
        tmp.writeBytes(bytes)
        tmp.renameTo(File(dir, "$k.bin"))
        File(dir, "$k.name").writeText(name)
    }

    /** The recovery copy of the file at [uri], if one was left behind. */
    fun find(uri: Uri): Copy? {
        val k = key(uri)
        val bin = File(dir, "$k.bin")
        if (!bin.isFile || bin.length() == 0L) return null
        val name = runCatching { File(dir, "$k.name").readText() }.getOrNull() ?: return null
        return Copy(name, bin, bin.lastModified())
    }

    /** Forgets the copy (the file was saved, or the changes were thrown away on purpose). */
    fun clear(uri: Uri) {
        val k = key(uri)
        listOf("bin", "name", "tmp").forEach { File(dir, "$k.$it").delete() }
    }
}

/** How often unsaved edits are copied for recovery. */
const val RECOVERY_EVERY_MS = 30_000L

/** How soon after the first unsaved edit the first copy is made, and how often the loop looks. */
const val RECOVERY_FIRST_MS = 5_000L

/**
 * The open documents' "copy now" hooks: when the app goes to the background (where Android may
 * end it, or the person swipes it away), every document with unsaved edits is copied at once.
 */
object RecoveryHub {
    private val hooks = java.util.concurrent.CopyOnWriteArraySet<() -> Unit>()

    fun register(hook: () -> Unit) {
        hooks.add(hook)
    }

    fun unregister(hook: () -> Unit) {
        hooks.remove(hook)
    }

    /** Called when the app leaves the screen. */
    fun copyAll() {
        hooks.forEach { runCatching { it() } }
    }
}
