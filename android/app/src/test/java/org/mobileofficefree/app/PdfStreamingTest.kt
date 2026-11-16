package org.mobileofficefree.app

import java.io.ByteArrayInputStream
import java.io.IOException
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PdfStreamingTest {
    private fun stream(text: String) = ByteArrayInputStream(text.toByteArray(Charsets.ISO_8859_1))

    @Test
    fun findsAWordAnywhereInTheStream() {
        val body = "x".repeat(1000)
        assertTrue(stream("/Annots$body").containsAnyAscii("/Annots", "/ObjStm", chunk = 16))
        assertTrue(stream("$body/ObjStm").containsAnyAscii("/Annots", "/ObjStm", chunk = 16))
        assertFalse(stream(body).containsAnyAscii("/Annots", "/ObjStm", chunk = 16))
        assertFalse(stream("").containsAnyAscii("/Annots"))
    }

    @Test
    fun findsAWordSplitBetweenChunks() {
        for (offset in 0 until 40) {
            val text = "y".repeat(offset) + "/Annots" + "y".repeat(50)
            assertTrue("offset $offset", stream(text).containsAnyAscii("/Annots", chunk = 8))
        }
        // Almost the word, cut at every chunk border: not found.
        assertFalse(stream("/Annot ".repeat(30)).containsAnyAscii("/Annots", chunk = 5))
    }

    @Test
    fun outOfMemoryIsFoundThroughCauses() {
        assertTrue(OutOfMemoryError("Failed to allocate a 179774832 byte allocation").isOutOfMemory())
        assertTrue(IllegalStateException("wrapped", OutOfMemoryError()).isOutOfMemory())
        assertFalse(IOException("disk full").isOutOfMemory())
    }
}
