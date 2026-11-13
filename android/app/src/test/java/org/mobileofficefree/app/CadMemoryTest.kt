package org.mobileofficefree.app

import java.util.Locale
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class CadMemoryTest {
    private val mb = 1024L * 1024L

    @Test
    fun recognisesFormatsAsTheEngineDoes() {
        assertEquals(CadFormat.DWG, CadMemory.format("AC1032\u0000\u0000rest".toByteArray(Charsets.ISO_8859_1)))
        assertEquals(CadFormat.BINARY_DXF, CadMemory.format("AutoCAD Binary DXF\r\n\u001a\u0000...".toByteArray(Charsets.ISO_8859_1)))
        assertEquals(CadFormat.DXF, CadMemory.format("  0\nSECTION\n".toByteArray(Charsets.US_ASCII)))
        // "AC10" without two digits is not a DWG for the engine either.
        assertEquals(CadFormat.DXF, CadMemory.format("AC10xx\u0000".toByteArray(Charsets.ISO_8859_1)))
        assertEquals(CadFormat.DXF, CadMemory.format(ByteArray(0)))
    }

    @Test
    fun estimateFollowsTheMeasuredPeaks() {
        // Measured: 18.2 MB DWG -> 3.5 GB peak; 118 MB ASCII DXF -> 2.6 GB; 71 MB binary DXF -> 2.1 GB.
        val dwg = CadMemory.estimate(CadFormat.DWG, 18_247_222)
        assertTrue(dwg in 3400 * mb..3900 * mb)
        val dxf = CadMemory.estimate(CadFormat.DXF, 118_454_581)
        assertTrue(dxf in 2550 * mb..2900 * mb)
        val binary = CadMemory.estimate(CadFormat.BINARY_DXF, 71_000_000)
        assertTrue(binary in 2050 * mb..2300 * mb)
    }

    @Test
    fun asksOnlyWhenThePeakDoesNotFit() {
        val budget = CadMemory.budget(availMem = 3000 * mb, threshold = 216 * mb)
        assertEquals(3000 * mb - CadMemory.MIN_MARGIN_BYTES, budget)
        assertFalse(CadMemory.tooLarge(CadMemory.estimate(CadFormat.DWG, 5_000_000), budget))
        assertTrue(CadMemory.tooLarge(CadMemory.estimate(CadFormat.DWG, 18_000_000), budget))
        // The system's own low-memory line counts when it is the larger margin.
        assertEquals(1000 * mb, CadMemory.budget(availMem = 1500 * mb, threshold = 500 * mb))
        // Already below that line: every drawing is asked about.
        assertTrue(CadMemory.tooLarge(CadMemory.estimate(CadFormat.DXF, 10_000), CadMemory.budget(400 * mb, 450 * mb)))
    }

    @Test
    fun readsThePeakResidentMemoryOrNothing() {
        // On Linux (and Android) the process's own peak is there; elsewhere nothing is read.
        if (java.io.File("/proc/self/status").exists()) assertTrue(CadMemory.peakResident() > mb) else assertEquals(0L, CadMemory.peakResident())
    }

    @Test
    fun labelsInMegabytesOrGigabytes() {
        assertEquals("850 MB", CadMemory.label(850 * mb, Locale.ROOT))
        assertEquals("3.4 GB", CadMemory.label((3.4 * 1024 * mb).toLong(), Locale.ROOT))
        assertEquals("3,4 GB", CadMemory.label((3.4 * 1024 * mb).toLong(), Locale.forLanguageTag("tr")))
    }
}
