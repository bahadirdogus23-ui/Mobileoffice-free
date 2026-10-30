package org.mobileofficefree.app

import java.io.ByteArrayOutputStream
import java.nio.charset.Charset
import java.nio.file.Files
import java.util.zip.ZipEntry
import java.util.zip.ZipException
import java.util.zip.ZipOutputStream
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
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
        val cp857 = Charset.forName("IBM857")
        // Without the UTF-8 flag, as Windows' "Send to compressed folder" writes them.
        val bytes = zip(cp857, "Çağrı Ödemesi.xlsx" to "x")
        val (entries, cs) = ArchiveReader.list { bytes.inputStream() }
        assertEquals(cp857, cs)
        assertEquals("Çağrı Ödemesi.xlsx", entries.single().name)
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
