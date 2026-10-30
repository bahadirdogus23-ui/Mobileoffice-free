package org.mobileofficefree.app

import androidx.compose.ui.geometry.Offset
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class SignatureTest {
    @Test
    fun signatureIsCentredOnTheTapAndAThirdOfThePageWide() {
        // A 200 × 40 px line drawn somewhere on the pad.
        val drawn = listOf(listOf(Offset(100f, 50f), Offset(300f, 90f)))
        val placed = placeSignature(drawn, at = Offset(300f, 400f), pageWidth = 600f, pageHeight = 800f)
        assertEquals(1, placed.size)
        val (x0, y0, x1, y1) = placed[0]
        // 200 pt wide (a third of 600), centred on x 300 → 200…400 pt → 1/3…2/3.
        assertEquals(1.0 / 3, x0, 1e-6)
        assertEquals(2.0 / 3, x1, 1e-6)
        // The same scale down: 40 px → 40 pt, centred on y 400 → 380…420 of 800.
        assertEquals(380.0 / 800, y0, 1e-6)
        assertEquals(420.0 / 800, y1, 1e-6)
    }

    @Test
    fun staysOnThePageNearAnEdge() {
        val drawn = listOf(listOf(Offset(0f, 0f), Offset(250f, 100f)), emptyList())
        val placed = placeSignature(drawn, at = Offset(0f, 0f), pageWidth = 600f, pageHeight = 800f)
        assertEquals(1, placed.size)
        assertTrue(placed[0].all { it in 0.0..1.0 })
        assertTrue(placeSignature(emptyList(), Offset.Zero, 600f, 800f).isEmpty())
    }
}
