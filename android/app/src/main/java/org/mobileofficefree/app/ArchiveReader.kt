package org.mobileofficefree.app

import java.io.File
import java.io.InputStream
import java.nio.ByteBuffer
import java.nio.CharBuffer
import java.nio.charset.CharacterCodingException
import java.nio.charset.Charset
import java.nio.charset.CharsetDecoder
import java.nio.charset.CharsetEncoder
import java.nio.charset.CoderResult
import java.nio.charset.CodingErrorAction
import java.util.zip.CRC32
import java.util.zip.ZipEntry
import java.util.zip.ZipException
import java.util.zip.ZipInputStream

/** A file inside an archive: its position in the archive, full path and unpacked size (-1 if unknown). */
data class ArchiveEntry(val index: Int, val path: String, val size: Long) {
    val name: String get() = path.trimEnd('/').substringAfterLast('/')
    val folder: String get() = path.trimEnd('/').substringBeforeLast('/', "")
}

/**
 * IBM code page 857 (DOS Turkish), the code page Windows' "Send to → Compressed folder" and old
 * WinZip write Turkish file names in. Built in, because not every Android has "IBM857" among its
 * charsets (without it names fell back to CP437 or Latin-1: "Çağrı" read as "Ça§r?"). It reads
 * Western DOS names (CP437, CP850) right too, except the rare ì ÿ ª º and box drawing, whose bytes
 * are Turkish letters here. Unassigned bytes (0xD5, 0xE7, 0xF2) read as U+FFFD.
 */
internal object Cp857 : Charset("x-mobileoffice-cp857", emptyArray<String>()) {
    private val high: CharArray = intArrayOf(
        0x00C7, 0x00FC, 0x00E9, 0x00E2, 0x00E4, 0x00E0, 0x00E5, 0x00E7, 0x00EA, 0x00EB, 0x00E8, 0x00EF, 0x00EE, 0x0131, 0x00C4, 0x00C5,
        0x00C9, 0x00E6, 0x00C6, 0x00F4, 0x00F6, 0x00F2, 0x00FB, 0x00F9, 0x0130, 0x00D6, 0x00DC, 0x00F8, 0x00A3, 0x00D8, 0x015E, 0x015F,
        0x00E1, 0x00ED, 0x00F3, 0x00FA, 0x00F1, 0x00D1, 0x011E, 0x011F, 0x00BF, 0x00AE, 0x00AC, 0x00BD, 0x00BC, 0x00A1, 0x00AB, 0x00BB,
        0x2591, 0x2592, 0x2593, 0x2502, 0x2524, 0x00C1, 0x00C2, 0x00C0, 0x00A9, 0x2563, 0x2551, 0x2557, 0x255D, 0x00A2, 0x00A5, 0x2510,
        0x2514, 0x2534, 0x252C, 0x251C, 0x2500, 0x253C, 0x00E3, 0x00C3, 0x255A, 0x2554, 0x2569, 0x2566, 0x2560, 0x2550, 0x256C, 0x00A4,
        0x00BA, 0x00AA, 0x00CA, 0x00CB, 0x00C8, 0xFFFD, 0x00CD, 0x00CE, 0x00CF, 0x2518, 0x250C, 0x2588, 0x2584, 0x00A6, 0x00CC, 0x2580,
        0x00D3, 0x00DF, 0x00D4, 0x00D2, 0x00F5, 0x00D5, 0x00B5, 0xFFFD, 0x00D7, 0x00DA, 0x00DB, 0x00D9, 0x00EC, 0x00FF, 0x00AF, 0x00B4,
        0x00AD, 0x00B1, 0xFFFD, 0x00BE, 0x00B6, 0x00A7, 0x00F7, 0x00B8, 0x00B0, 0x00A8, 0x00B7, 0x00B9, 0x00B3, 0x00B2, 0x25A0, 0x00A0,
    ).map { it.toChar() }.toCharArray()

    private val byChar: Map<Char, Int> by lazy {
        high.withIndex().filter { it.value != '�' }.associate { it.value to 0x80 + it.index }
    }

    override fun contains(cs: Charset): Boolean = cs == this || cs == Charsets.US_ASCII

    override fun newDecoder(): CharsetDecoder = object : CharsetDecoder(this, 1f, 1f) {
        override fun decodeLoop(src: ByteBuffer, dst: CharBuffer): CoderResult {
            while (src.hasRemaining()) {
                if (!dst.hasRemaining()) return CoderResult.OVERFLOW
                val b = src.get().toInt() and 0xFF
                dst.put(if (b < 0x80) b.toChar() else high[b - 0x80])
            }
            return CoderResult.UNDERFLOW
        }
    }

    override fun newEncoder(): CharsetEncoder = object : CharsetEncoder(this, 1f, 1f) {
        override fun encodeLoop(src: CharBuffer, dst: ByteBuffer): CoderResult {
            while (src.hasRemaining()) {
                val c = src.get(src.position())
                val b = if (c.code < 0x80) c.code else byChar[c] ?: return CoderResult.unmappableForLength(1)
                if (!dst.hasRemaining()) return CoderResult.OVERFLOW
                dst.put(b.toByte())
                src.position(src.position() + 1)
            }
            return CoderResult.UNDERFLOW
        }
    }
}

/**
 * ZIP reading for the archive viewer. Names written by Windows without the UTF-8 flag are in the
 * DOS code page of the machine that made them; for Turkish ones that is CP857, so "Dilekçe.docx"
 * comes out right instead of as "Dilek├ğe.docx". An Info-ZIP Unicode Path field (0x7075), which
 * 7-Zip and Info-ZIP add beside a DOS name, wins over the DOS name it was written for.
 */
object ArchiveReader {
    /** Name encodings to try, in order: UTF-8 rejects what it cannot read; CP857 reads anything. */
    val nameCharsets: List<Charset> = listOf(Charsets.UTF_8, Cp857)

    /**
     * The name an Info-ZIP Unicode Path extra field (header 0x7075: version 1, the CRC-32 of the
     * entry's own name bytes, the name in UTF-8) in [extra] gives, when its CRC matches one of
     * [rawNames] (the name as stored); a field written for another name (the entry was renamed by
     * a program that does not know the field) is ignored.
     */
    internal fun unicodePath(extra: ByteArray?, rawNames: List<ByteArray>): String? {
        if (extra == null) return null
        fun u16(at: Int) = (extra[at].toInt() and 0xFF) or ((extra[at + 1].toInt() and 0xFF) shl 8)
        var at = 0
        while (at + 4 <= extra.size) {
            val id = u16(at)
            val size = u16(at + 2)
            val start = at + 4
            if (start + size > extra.size) return null
            if (id == 0x7075 && size >= 5 && extra[start].toInt() == 1) {
                var crc = 0L
                for (i in 0 until 4) crc = crc or ((extra[start + 1 + i].toLong() and 0xFF) shl (8 * i))
                val matches = rawNames.any { raw -> CRC32().apply { update(raw) }.value == crc }
                if (!matches) return null
                val strict = Charsets.UTF_8.newDecoder()
                    .onMalformedInput(CodingErrorAction.REPORT)
                    .onUnmappableCharacter(CodingErrorAction.REPORT)
                return try {
                    strict.decode(ByteBuffer.wrap(extra, start + 5, size - 5)).toString().takeIf { it.isNotEmpty() }
                } catch (_: CharacterCodingException) {
                    null
                }
            }
            at = start + size
        }
        return null
    }

    /** The bytes [name] may have been stored as: in [cs] (when it can be), and in UTF-8 (entries flagged UTF-8). */
    private fun storedForms(name: String, cs: Charset): List<ByteArray> {
        val inCs = try {
            val out = cs.newEncoder()
                .onMalformedInput(CodingErrorAction.REPORT)
                .onUnmappableCharacter(CodingErrorAction.REPORT)
                .encode(CharBuffer.wrap(name))
            ByteArray(out.remaining()).also { out.get(it) }
        } catch (_: CharacterCodingException) {
            null
        }
        return listOfNotNull(inCs, name.toByteArray(Charsets.UTF_8))
    }

    /** An entry's name: its Unicode Path field when it has a valid one, else the name as read with [cs]. */
    private fun nameOf(e: ZipEntry, cs: Charset): String = unicodePath(e.extra, storedForms(e.name, cs)) ?: e.name

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
                        out += ArchiveEntry(index, nameOf(e, cs).replace('\\', '/'), size)
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
                    val name = safeName(nameOf(e, cs).replace('\\', '/').substringAfterLast('/'))
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
