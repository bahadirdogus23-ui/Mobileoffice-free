package org.mobileofficefree.app

import org.junit.Assert.assertEquals
import org.junit.Test

class TextSizeStepTest {
    @Test
    fun sizesStepAsOfficeDoes() {
        assertEquals(11f, stepSize(10f, 1))
        assertEquals(14f, stepSize(12f, 1))
        assertEquals(11f, stepSize(12f, -1))
        assertEquals(32f, stepSize(28f, 1))
        assertEquals(26f, stepSize(28f, -1))
        assertEquals(80f, stepSize(72f, 1))
        assertEquals(68f, stepSize(72f, -1))
        // Office's range: 4 to 144 points.
        assertEquals(4f, stepSize(4f, -1))
        assertEquals(144f, stepSize(144f, 1))
    }
}
