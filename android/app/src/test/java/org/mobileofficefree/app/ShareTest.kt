package org.mobileofficefree.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test

class ShareTest {
    private val note = "… (kısaltıldı)"

    @Test
    fun shortTextIsLeftAlone() {
        val text = "Ölçümler\nsatır 2"
        assertSame(text, Share.clipText(text, 100, note))
    }

    @Test
    fun longTextIsCutAtALineEndWithTheNote() {
        val text = (1..1000).joinToString("\n") { "satır $it: ığüşöç İ" }
        val cut = Share.clipText(text, 500, note)
        assertTrue(cut.length <= 500)
        assertTrue(cut.endsWith("\n\n$note"))
        val kept = cut.removeSuffix("\n\n$note")
        assertTrue(text.startsWith(kept))
        // Whole lines only.
        assertEquals('\n', text[kept.length])
    }

    @Test
    fun neverSplitsASurrogatePair() {
        val text = "a".repeat(10) + "😀".repeat(200)
        for (max in 30..60) {
            val cut = Share.clipText(text, max, note).removeSuffix("\n\n$note")
            assertFalse(cut.isNotEmpty() && Character.isHighSurrogate(cut.last()))
            assertTrue(text.startsWith(cut))
        }
    }

    @Test
    fun theDefaultLimitStaysWellUnderTheBinderBuffer() {
        // UTF-16, and the chooser keeps a second copy in its ClipData.
        assertTrue(Share.MAX_TEXT_CHARS * 2 * 2 <= 256 * 1024)
        val report = "x".repeat(300_000)
        assertTrue(Share.clipText(report, note = note).length <= Share.MAX_TEXT_CHARS)
    }
}
