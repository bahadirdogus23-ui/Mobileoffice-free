package org.mobileofficefree.app

import java.io.File
import java.io.InputStream
import java.nio.charset.Charset
import java.util.zip.ZipException
import java.util.zip.ZipInputStream

/** A file inside an archive: its position in the archive, full path and unpacked size (-1 if unknown). */
data class ArchiveEntry(val index: Int, val path: String, val size: Long) {
    val name: String get() = path.trimEnd('/').substringAfterLast('/')
    val folder: String get() = path.trimEnd('/').substringBeforeLast('/', "")
}

/**
 * ZIP reading for the archive viewer. Names written by Windows without the UTF-8 flag are in the
 * DOS code page of the machine that made them; for Turkish ones that is CP857, so "Dilekçe.docx"
 * comes out right instead of as "Dilek├ğe.docx".
 */
object ArchiveReader {
    /** Name encodings to try, in order: UTF-8 rejects what it cannot read; the DOS pages read anything. */
    val nameCharsets: List<Charset> =
        (listOf("UTF-8", "IBM857", "IBM437").filter(Charset::isSupported).map(Charset::forName) + Charsets.ISO_8859_1).distinct()

    /** Unpacking stops past this many bytes, so a "zip bomb" cannot fill the phone. */
    const val MAX_ENTRY_BYTES = 512L * 1024 * 1024
    const val MAX_ENTRIES = 20_000

    /** The files (not folders) in the ZIP that [open] streams, and the charset their names were read with. */
    fun list(open: () -> InputStream): Pair<List<ArchiveEntry>, Charset> {
        var last: Exception? = null
        for (cs in nameCharsets) {
            try {
                return listWith(open, cs) to cs
            } catch (e: IllegalArgumentException) {
                last = e // A name is not valid in this charset: try the next one.
            }
        }
        throw last ?: ZipException("unreadable archive")
    }

    private fun listWith(open: () -> InputStream, cs: Charset): List<ArchiveEntry> {
        val out = ArrayList<ArchiveEntry>()
        open().use { raw ->
            ZipInputStream(raw.buffered(), cs).use { zip ->
                var index = 0
                while (out.size < MAX_ENTRIES) {
                    val e = zip.nextEntry ?: break
                    if (!e.isDirectory) {
                        // Streamed ZIPs only state sizes after the data; count it then.
                        val size = if (e.size >= 0) e.size else countBytes(zip)
                        out += ArchiveEntry(index, e.name.replace('\\', '/'), size)
                    }
                    index++
                }
            }
        }
        if (out.isEmpty()) {
            // ZipInputStream reads garbage as "no entries"; an archive with nothing in it is not worth showing either.
            throw ZipException("no files in archive")
        }
        return out
    }

    /** Unpacks entry number [index] into [dir] under a safe file name, returning the file. */
    fun extract(open: () -> InputStream, cs: Charset, index: Int, dir: File): File {
        open().use { raw ->
            ZipInputStream(raw.buffered(), cs).use { zip ->
                var i = 0
                while (true) {
                    val e = zip.nextEntry ?: throw ZipException("entry $index not found")
                    if (i++ != index) continue
                    dir.mkdirs()
                    // Only the last path part, cleaned: entries such as "../../x" must not leave [dir].
                    val name = safeName(e.name.replace('\\', '/').substringAfterLast('/'))
                    val file = File(dir, name)
                    file.outputStream().use { out ->
                        val buf = ByteArray(64 * 1024)
                        var total = 0L
                        while (true) {
                            val n = zip.read(buf)
                            if (n < 0) break
                            total += n
                            if (total > MAX_ENTRY_BYTES) {
                                out.close()
                                file.delete()
                                throw ZipException("entry larger than ${MAX_ENTRY_BYTES / (1024 * 1024)} MB")
                            }
                            out.write(buf, 0, n)
                        }
                    }
                    return file
                }
            }
        }
    }

    internal fun safeName(name: String): String {
        val cleaned = name.replace(Regex("[\\u0000-\\u001f/\\\\:*?\"<>|]"), "_").trimStart('.').trim()
        return cleaned.ifEmpty { "file" }.take(120)
    }

    private fun countBytes(zip: ZipInputStream): Long {
        val buf = ByteArray(64 * 1024)
        var total = 0L
        while (true) {
            val n = zip.read(buf)
            if (n < 0) return total
            total += n
        }
    }
}
