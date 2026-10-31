package org.mobileofficefree.app

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class GuideTest {
    @Test
    fun partsHaveTitlesAndLines() {
        val parts = guideParts("# Bir\nyazı\n- madde\n\n# İki\n- öbür")
        assertEquals(listOf("Bir", "İki"), parts.map { it.title })
        assertEquals(listOf("yazı", "- madde"), parts[0].lines)
    }

    @Test
    fun bothGuidesHaveTheSameParts() {
        // Unit tests run in the module directory.
        val tr = guideParts(File("src/main/res/raw-tr/guide.txt").readText())
        val en = guideParts(File("src/main/res/raw/guide.txt").readText())
        assertTrue(tr.size >= 8)
        assertEquals(tr.size, en.size)
        assertEquals(tr.map { it.lines.size }, en.map { it.lines.size })
    }
}
