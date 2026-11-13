package org.mobileofficefree.app

import java.io.File
import java.io.IOException
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class RecoveryTest {
    @get:Rule val folder = TemporaryFolder()

    @Test
    fun replacesPreviousCopyWithCompleteNewCopy() {
        val destination = folder.newFile("copy.bin").apply { writeText("old") }
        val temporary = folder.newFile("copy.tmp").apply { writeText("new complete document") }
        replaceRecoveryCopy(temporary, destination)
        assertEquals("new complete document", destination.readText())
        assertFalse(temporary.exists())
    }

    @Test
    fun missingTemporaryCopyDoesNotErasePreviousCopyOrReportSuccess() {
        val destination = folder.newFile("copy.bin").apply { writeText("previous recovery") }
        assertThrows(IOException::class.java) { replaceRecoveryCopy(File(folder.root, "missing.tmp"), destination) }
        assertEquals("previous recovery", destination.readText())
    }

    @Test
    fun failedReplacementKeepsTemporaryCopyAndReportsFailure() {
        val destination = folder.newFolder("copy.bin")
        File(destination, "existing").writeText("keep")
        val temporary = folder.newFile("copy.tmp").apply { writeText("new recovery") }
        assertThrows(IOException::class.java) { replaceRecoveryCopy(temporary, destination) }
        assertEquals("new recovery", temporary.readText())
        assertEquals("keep", File(destination, "existing").readText())
    }
}
