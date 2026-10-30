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
    }

    @Test
    fun decodesUtf8AndFallsBackToTurkishWindowsCodePage() {
        val turkish = "Şişli'de ığdır çağrı"
        assertEquals(turkish to "UTF-8", decodeText(turkish.toByteArray(Charsets.UTF_8)))
        val cp1254 = turkish.toByteArray(charset("windows-1254"))
        assertEquals(turkish to "Windows-1254", decodeText(cp1254))
        val bom = byteArrayOf(0xEF.toByte(), 0xBB.toByte(), 0xBF.toByte()) + "ğ".toByteArray()
        assertEquals("ğ" to "UTF-8 (BOM)", decodeText(bom))
        val utf16 = byteArrayOf(0xFF.toByte(), 0xFE.toByte()) + "İ".toByteArray(Charsets.UTF_16LE)
        assertEquals("İ" to "UTF-16", decodeText(utf16))
    }

    @Test
    fun aCutNeverSplitsALetter() {
        val bytes = "aaaş".toByteArray() // 'ş' is two bytes
        val end = bytes.size - 1 // in the middle of 'ş'
        val cut = cutBeforeLastLetter(bytes, end)
        assertEquals(3, cut)
        assertEquals("aaa" to "UTF-8", decodeText(bytes.copyOf(cut)))
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
    }

    @Test
    fun putsWindowsLineEndsBack() {
        assertEquals("a\r\nb\r\n", withLineEnds("a\nb\n", crlf = true))
        assertEquals("a\nb", withLineEnds("a\nb", crlf = false))
        assertEquals("a\r\nb", withLineEnds("a\r\nb", crlf = true))
    }
}
