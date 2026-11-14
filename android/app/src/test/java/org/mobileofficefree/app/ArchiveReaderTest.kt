package org.mobileofficefree.app

import java.io.ByteArrayOutputStream
import java.nio.charset.Charset
import java.nio.file.Files
import java.util.zip.CRC32
import java.util.zip.ZipEntry
import java.util.zip.ZipException
import java.util.zip.ZipOutputStream
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ArchiveReaderTest {
    private fun zip(cs: Charset, vararg files: Pair<String, String>): ByteArray {
        val out = ByteArrayOutputStream()
        ZipOutputStream(out, cs).use { z ->
            for ((name, body) in files) {
                z.putNextEntry(ZipEntry(name))
                z.write(body.toByteArray())
                z.closeEntry()
            }
        }
        return out.toByteArray()
    }

    @Test
    fun readsUtf8Names() {
        val bytes = zip(Charsets.UTF_8, "Belgeler/Dilekçe.docx" to "x", "ışık.txt" to "merhaba")
        val (entries, cs) = ArchiveReader.list { bytes.inputStream() }
        assertEquals(Charsets.UTF_8, cs)
        assertEquals(listOf("Dilekçe.docx", "ışık.txt"), entries.map { it.name })
        assertEquals("Belgeler", entries[0].folder)
        assertEquals(7L, entries[1].size)
    }

    @Test
    fun readsTurkishWindowsNames() {
        // Without the UTF-8 flag, as Windows' "Send to compressed folder" writes them.
        val bytes = zip(Charset.forName("IBM857"), "İıŞşĞğÇçÖöÜü/Çağrı Ödemesi.xlsx" to "x")
        val (entries, cs) = ArchiveReader.list { bytes.inputStream() }
        assertEquals(Cp857, cs)
        assertEquals("Çağrı Ödemesi.xlsx", entries.single().name)
        assertEquals("İıŞşĞğÇçÖöÜü", entries.single().folder)
        val dir = Files.createTempDirectory("arc").toFile()
        assertEquals("Çağrı Ödemesi.xlsx", ArchiveReader.extract({ bytes.inputStream() }, cs, entries.single().index, dir).name)
        dir.deleteRecursively()
    }

    @Test
    fun builtInCp857MatchesTheJavaTableAndReadsWesternNames() {
        val java = Charset.forName("IBM857")
        val all = ByteArray(256) { it.toByte() }
        val ours = String(all, Cp857)
        val theirs = String(all, java)
        for (i in 0 until 256) {
            if (ours[i] != '�') assertEquals("byte $i", theirs[i], ours[i])
        }
        assertEquals("İıŞşĞğÇçÖöÜü", String(intArrayOf(0x98, 0x8D, 0x9E, 0x9F, 0xA6, 0xA7, 0x80, 0x87, 0x99, 0x94, 0x9A, 0x81).map { it.toByte() }.toByteArray(), Cp857))
        assertEquals("Ğ".toByteArray(java).toList(), "Ğ".toByteArray(Cp857).toList())
        // A Western Windows name (CP850) keeps its letters: é ü ç ñ á are the same bytes.
        val bytes = zip(Charset.forName("IBM850"), "Café Menü Señor Ação.txt" to "x")
        assertEquals("Café Menü Señor Ação.txt", ArchiveReader.list { bytes.inputStream() }.first.single().name)
    }

    /** An Info-ZIP Unicode Path extra field (0x7075) for an entry stored under [stored], giving [unicode]. */
    private fun unicodePathField(stored: ByteArray, unicode: String): ByteArray {
        val name = unicode.toByteArray(Charsets.UTF_8)
        val crc = CRC32().apply { update(stored) }.value
        val size = 5 + name.size
        return byteArrayOf(0x75, 0x70, size.toByte(), (size shr 8).toByte(), 1) +
            ByteArray(4) { (crc shr (8 * it)).toByte() } + name
    }

    @Test
    fun theUnicodePathFieldWinsOverTheDosName() {
        // 7-Zip and Info-ZIP store a DOS name ("?" for what the code page lacks) and the real one.
        val out = ByteArrayOutputStream()
        ZipOutputStream(out, Charsets.ISO_8859_1).use { z ->
            for ((stored, real) in listOf("Rapor ?ubat.txt" to "Rapor Şubat.txt", "eski.txt" to "Başka İsim.txt")) {
                val e = ZipEntry(stored)
                // The second field was written for another name (the entry was renamed since): ignored.
                val crcOf = if (stored == "eski.txt") "yeni.txt" else stored
                e.extra = unicodePathField(crcOf.toByteArray(Charsets.ISO_8859_1), real)
                z.putNextEntry(e)
                z.write("x".toByteArray())
                z.closeEntry()
            }
        }
        val bytes = out.toByteArray()
        val (entries, cs) = ArchiveReader.list { bytes.inputStream() }
        assertEquals(listOf("Rapor Şubat.txt", "eski.txt"), entries.map { it.name })
        val dir = Files.createTempDirectory("arc").toFile()
        assertEquals("Rapor Şubat.txt", ArchiveReader.extract({ bytes.inputStream() }, cs, entries[0].index, dir).name)
        dir.deleteRecursively()
        assertNull(ArchiveReader.unicodePath(null, emptyList()))
        assertNull(ArchiveReader.unicodePath(byteArrayOf(0x75, 0x70, 0x40, 0), listOf(byteArrayOf())))
    }

    @Test
    fun extractsOneEntryInsideTheGivenFolderOnly() {
        val bytes = zip(Charsets.UTF_8, "a.txt" to "bir", "../../kacis.txt" to "iki")
        val (entries, cs) = ArchiveReader.list { bytes.inputStream() }
        val dir = Files.createTempDirectory("arc").toFile()
        val file = ArchiveReader.extract({ bytes.inputStream() }, cs, entries[1].index, dir)
        assertEquals(dir, file.parentFile)
        assertEquals("kacis.txt", file.name)
        assertEquals("iki", file.readText())
        assertFalse(java.io.File(dir.parentFile?.parentFile, "kacis.txt").exists())
        dir.deleteRecursively()
    }

    @Test
    fun cleansDangerousNames() {
        assertEquals("passwd", ArchiveReader.safeName("..passwd"))
        assertEquals("a_b", ArchiveReader.safeName("a:b"))
        assertEquals("file", ArchiveReader.safeName(".."))
    }

    @Test
    fun rejectsSomethingThatIsNotAZip() {
        val failed = runCatching { ArchiveReader.list { "not a zip".byteInputStream() } }
        assertTrue(failed.exceptionOrNull() is ZipException)
    }
}
