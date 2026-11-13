package org.mobileofficefree.app

import java.io.File
import java.io.IOException
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class MeasurementRecordsTest {
    @get:Rule val folder = TemporaryFolder()

    @Test fun damagedRecordsDoNotHideLaterDurationAndFrameRecords() {
        val lines = listOf(
            "S\tPDF\t2\t120\t40\t80\t80\t0",
            "S\tbroken\t2\tnot-a-number\t40\t80\t80\t0",
            "J\tAna ekran\t10\t2\t1\t300",
            "S\ttruncated\t2\t120",
            "S\tExcel\t1\t40\t40\t40\t40",
        )
        val records = lines.mapNotNull(::parseMeasurementRecord)
        assertEquals(listOf("PDF", "Ana ekran", "Excel"), records.map { it.name })
        assertEquals(listOf(10L, 2L, 1L, 300L), records[1].values)
        assertEquals(5, records[2].values.size) // Legacy record without a slow count.
        assertNull(parseMeasurementRecord("S\tbad\t-1\t0\t0\t0\t0"))
        assertNull(parseMeasurementRecord("J\tbad\t99999999999999999999\t0\t0\t0"))
    }

    @Test fun snapshotIsReplacedCompletelyAndCanBeReadBack() {
        val file = folder.newFile("stats.txt").apply { writeText("old snapshot") }
        val text = "S\tPDF\t2\t120\t40\t80\t80\t0\nJ\tAna ekran\t10\t2\t1\t300"
        writeMeasurementSnapshot(file, text)
        assertEquals(text, file.readText())
        assertEquals(2, file.readLines().mapNotNull(::parseMeasurementRecord).size)
        assertFalse(File(folder.root, "stats.txt.tmp").exists())
    }

    @Test fun failedReplacementReportsFailureAndPreservesPreviousData() {
        val target = folder.newFolder("stats.txt")
        File(target, "existing").writeText("keep")
        assertThrows(IOException::class.java) { writeMeasurementSnapshot(target, "new snapshot") }
        assertEquals("keep", File(target, "existing").readText())
        assertEquals("new snapshot", File(folder.root, "stats.txt.tmp").readText())
    }
}
