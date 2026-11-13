package org.mobileofficefree.app

import android.content.Context
import android.net.Uri
import android.os.SystemClock
import java.io.File
import java.security.MessageDigest
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.coroutines.yield

/**
 * Searching inside files: each document's words are read once with the engine that opens it and
 * kept in the app's own folder (by its address, size and date), so later searches only read that
 * text. Nothing leaves the phone; the person's files are only read.
 */
object ContentSearch {
    /** Files larger than this are not read for searching (bytes), nor more text than [MAX_CHARS]. */
    private const val MAX_FILE = 40L * 1024 * 1024
    private const val MAX_CHARS = 2_000_000

    /** One file the search may read: its address, name, size and date (for the kept text). */
    class Doc(val uri: Uri, val name: String, val size: Long, val date: Long)

    /** A file where the words were found, with a little of the text around them. */
    class Hit(val uri: Uri, val name: String, val snippet: String)

    /** Engine work runs on one thread of its own, never on the screen's. */
    @OptIn(ExperimentalCoroutinesApi::class)
    private val reader = Dispatchers.IO.limitedParallelism(1)

    private fun dir(context: Context) = File(context.filesDir, "search-index").apply { mkdirs() }

    private fun key(d: Doc): String =
        MessageDigest.getInstance("SHA-1").digest("${d.uri}|${d.size}|${d.date}".toByteArray()).joinToString("") { "%02x".format(it) }

    /** The words of [d], from the kept text or read now (null when it cannot be read). */
    private suspend fun textOf(context: Context, d: Doc): String? = withContext(reader) {
        val kept = File(dir(context), key(d) + ".txt")
        if (kept.isFile) return@withContext runCatching { kept.readText() }.getOrNull()
        if (d.size > MAX_FILE) return@withContext null
        val started = SystemClock.elapsedRealtime()
        val text = runCatching {
            val bytes = context.contentResolver.openInputStream(d.uri)?.use { it.readBytes() } ?: return@runCatching null
            org.mobileofficefree.engine.documentText(d.name, bytes, MAX_CHARS.toUInt())
        }.getOrNull()
        Measurements.sample("İçerik arama: dosya okuma", SystemClock.elapsedRealtime() - started)
        // Kept even when empty, so a file without words is not read again.
        runCatching { kept.writeText(text.orEmpty()) }
        text
    }

    /**
     * The files among [docs] whose words contain [query] (case, accents and Turkish ı/i folded),
     * at most [limit]; [progress] hears how many were looked at.
     */
    suspend fun search(context: Context, docs: List<Doc>, query: String, limit: Int = 100, progress: (Int, Int) -> Unit): List<Hit> {
        val needle = query.trim().searchFold()
        if (needle.length < 2) return emptyList()
        val hits = ArrayList<Hit>()
        for ((i, d) in docs.withIndex()) {
            progress(i + 1, docs.size)
            yield()
            val text = textOf(context, d) ?: continue
            // Folding up to MAX_CHARS of text is real work: never on the screen's thread.
            val snippet = withContext(Dispatchers.Default) {
                val folded = text.searchFold()
                val at = folded.indexOf(needle)
                if (at < 0) return@withContext null
                // Folding keeps lengths for Turkish text, so the place maps back to the original.
                val from = (at - 40).coerceAtLeast(0)
                val to = (at + needle.length + 60).coerceAtMost(text.length)
                (if (from > 0) "…" else "") + text.substring(from, to).replace('\n', ' ').trim() + (if (to < text.length) "…" else "")
            } ?: continue
            hits += Hit(d.uri, d.name, snippet)
            if (hits.size >= limit) break
        }
        return hits
    }

    /** Forgets the kept text of every file (Settings, or to save space). */
    fun clear(context: Context) {
        dir(context).deleteRecursively()
    }
}
