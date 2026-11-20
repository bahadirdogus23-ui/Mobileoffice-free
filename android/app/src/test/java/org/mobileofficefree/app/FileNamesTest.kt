package org.mobileofficefree.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class FileNamesTest {
    @Test
    fun validNamesPass() {
        assertNull(FileNames.problem("Dilekçe İŞ 2026.udf"))
        assertNull(FileNames.problem("  rapor.pdf  "))
        assertNull(FileNames.problem("no-extension"))
    }

    @Test
    fun badNamesAreRefused() {
        assertEquals(FileNames.Problem.EMPTY, FileNames.problem("   "))
        for (c in FileNames.ILLEGAL_CHARS) assertEquals("$c", FileNames.Problem.ILLEGAL, FileNames.problem("a${c}b.pdf"))
        assertEquals(FileNames.Problem.ILLEGAL, FileNames.problem("a\nb.pdf"))
        assertEquals(FileNames.Problem.HIDDEN, FileNames.problem(".pdf"))
        assertEquals(FileNames.Problem.HIDDEN, FileNames.problem(".."))
        // 128 two-byte letters: 256 bytes.
        assertEquals(FileNames.Problem.TOO_LONG, FileNames.problem("ş".repeat(128)))
    }

    @Test
    fun extensionAndBase() {
        assertEquals("docx", FileNames.extension("Rapor.v2.docx"))
        assertEquals("Rapor.v2", FileNames.base("Rapor.v2.docx"))
        assertEquals("", FileNames.extension(".gizli"))
        assertEquals(".gizli", FileNames.base(".gizli"))
        assertEquals("", FileNames.extension("ad."))
        assertEquals("", FileNames.extension("ad"))
    }

    @Test
    fun extensionChangeIsCaseInsensitiveAndTurkishSafe() {
        // Under a Turkish default locale "ZIP".lowercase() would be "zıp"; Locale.ROOT keeps it "zip".
        assertFalse(FileNames.extensionChanged("ARŞİV.ZIP", "Arşiv yeni.zip"))
        assertTrue(FileNames.extensionChanged("a.docx", "a.doc"))
        assertTrue(FileNames.extensionChanged("a.docx", "a"))
    }

    @Test
    fun copyNames() {
        assertEquals("Rapor (kopya).docx", FileNames.copyName("Rapor.docx", "kopya"))
        assertEquals("Rapor (kopya 2).docx", FileNames.copyName("Rapor.docx", "kopya", 2))
        assertEquals("notlar (copy)", FileNames.copyName("notlar", "copy"))
        val taken = setOf("Rapor (kopya).docx", "Rapor (kopya 2).docx")
        assertEquals("Rapor (kopya 3).docx", FileNames.uniqueCopyName("Rapor.docx", "kopya") { it in taken })
    }
}
