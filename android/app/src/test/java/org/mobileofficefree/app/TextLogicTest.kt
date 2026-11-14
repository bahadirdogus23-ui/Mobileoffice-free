package org.mobileofficefree.app

import java.io.ByteArrayOutputStream
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class TextLogicTest {
    @Test
    fun searchFoldMergesTurkishLettersAndKeepsLength() {
        val words = listOf("ışık", "IŞIK", "isik", "İSİK", "Işık")
        assertEquals(1, words.map { it.searchFold() }.toSet().size)
        assertEquals("isik", "IŞIK".searchFold())
        assertTrue("Çağrı Merkezi Raporu.docx".searchFold().contains("cagri"))
        for (w in words + "Göğüş çığlığı ÖÜÇ") assertEquals(w.length, w.searchFold().length)
        // "istanbul" finds the city however it was written: Turkish capitals, English spelling.
        for (w in listOf("İSTANBUL", "Istanbul", "ISTANBUL", "ıstanbul")) assertEquals(w, "istanbul", w.searchFold())
    }

    @Test
    fun fileEncodingsShowThePickersLabels() {
        // Names as the native decoder gives them (decoding itself is tested in tools/jvm-smoke).
        assertEquals("Windows-1254", FileEncoding("windows-1254", bom = false, exact = true).label)
        assertEquals("UTF-8 (BOM)", FileEncoding("UTF-8", bom = true, exact = true).label)
        assertEquals("UTF-16", FileEncoding("UTF-16LE", bom = true, exact = true).label)
        assertEquals("UTF-16LE", FileEncoding("UTF-16LE", bom = false, exact = true).label)
        assertEquals("windows-1251", FileEncoding("windows-1251", bom = false, exact = true).label)
        assertTrue(TextEncoding.UTF16BE.matches(FileEncoding("UTF-16BE", bom = true, exact = true)))
        assertTrue(!TextEncoding.UTF8.matches(FileEncoding("UTF-8", bom = true, exact = true)))
    }

    @Test
    fun splitsLinesAndSlicesVeryLongOnes() {
        val lines = splitLines("bir\r\niki\n\nüç\n")
        assertEquals(listOf("bir", "iki", "", "üç"), lines.map { it.text })
        assertEquals(listOf(1, 2, 3, 4), lines.map { it.number })
        val long = splitLines("x".repeat(LINE_SLICE * 2 + 5))
        assertEquals(3, long.size)
        assertEquals(listOf(false, true, true), long.map { it.continued })
        assertTrue(long.all { it.number == 1 })
    }

    @Test
    fun recognisesFilesByTheirFirstBytes() {
        fun sniff(bytes: ByteArray) = FileInfo.sniff(bytes.inputStream())
        assertEquals("pdf", sniff("%PDF-1.7\n".toByteArray()))
        assertEquals("png", sniff(byteArrayOf(0x89.toByte(), 0x50, 0x4E, 0x47, 0x0D, 0x0A)))
        assertEquals("dwg", sniff("AC1032\u0000\u0000".toByteArray()))
        assertEquals("docx", sniff(zip("[Content_Types].xml", "_rels/.rels", "word/document.xml")))
        assertEquals("xlsx", sniff(zip("[Content_Types].xml", "xl/workbook.xml")))
        assertEquals("pptx", sniff(zip("[Content_Types].xml", "ppt/presentation.xml")))
        assertNull(sniff("just text".toByteArray()))
        assertEquals("zip", sniff(zip("readme.txt")))
        // UYAP UDF: content.xml without an OpenDocument "mimetype" entry.
        assertEquals("udf", sniff(zip("content.xml", "documentproperties.xml")))
        assertNull(sniff(byteArrayOf(1, 2)))
    }

    private fun zip(vararg names: String): ByteArray {
        val out = ByteArrayOutputStream()
        ZipOutputStream(out).use { z ->
            for (n in names) {
                z.putNextEntry(ZipEntry(n))
                z.write("x".toByteArray())
                z.closeEntry()
            }
        }
        return out.toByteArray()
    }

    @Test
    fun savesInTheEncodingItCameIn() {
        val turkish = "Şişli'de ığdır çağrı İĞÜŞÖÇ"
        for (enc in TextEncoding.entries) {
            val (back, detected) = TextEncoding.detect(enc.encode(turkish))
            assertEquals(turkish, back)
            // Without a BOM, Windows-1254 and ISO-8859-9 read back as Windows-1254 (same letters).
            if (enc != TextEncoding.ISO_8859_9) assertEquals(enc, detected)
        }
    }

    @Test
    fun refusesLettersTheEncodingCannotHold() {
        val failed = runCatching { TextEncoding.WINDOWS_1254.encode("Merhaba 😀") }.exceptionOrNull()
        assertTrue(failed is UnencodableText)
        assertEquals("Windows-1254", (failed as UnencodableText).encoding)
        // The whole emoji, not half of its surrogate pair.
        assertEquals("😀", failed.char)
    }

    @Test
    fun putsWindowsLineEndsBack() {
        assertEquals("a\r\nb\r\n", withLineEnds("a\nb\n", crlf = true))
        assertEquals("a\nb", withLineEnds("a\nb", crlf = false))
        assertEquals("a\r\nb", withLineEnds("a\r\nb", crlf = true))
    }

    @Test
    fun repairsTurkishTextReadWithTheWrongCodePage() {
        assertEquals("İSTANBUL, şeker, dağı, ılık", repairTurkish("ÝSTANBUL, þeker, daðý, ýlýk"))
        val utf8AsLatin = String("çok güzel ğüşıöç İĞÜŞÖÇ".toByteArray(Charsets.UTF_8), charset("windows-1252"))
        assertEquals("çok güzel ğüşıöç İĞÜŞÖÇ", repairTurkish(utf8AsLatin))
        assertNull(repairTurkish("Her şey yolunda: ğüşıöç"))
    }
}
