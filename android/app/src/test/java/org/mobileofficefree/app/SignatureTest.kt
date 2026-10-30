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

    @Test
    fun keptSignaturesRoundTripScaledToOne() {
        val drawn = listOf(listOf(Offset(100f, 50f), Offset(300f, 90f)), listOf(Offset(150f, 60f)))
        val kept = SavedSignatures.normalized(drawn)
        assertEquals(Offset(0f, 0f), kept[0][0])
        assertEquals(Offset(1f, 0.2f), kept[0][1])
        val back = SavedSignatures.decode(SavedSignatures.encode(listOf(kept, kept)))
        assertEquals(listOf(kept, kept), back)
        assertTrue(SavedSignatures.decode("bozuk;satır\n").isEmpty())
    }

    @Test
    fun keepsAtMostThreeNewestFirst() {
        val file = java.io.File.createTempFile("imza", ".txt")
        try {
            val store = SavedSignatures(file)
            for (i in 1..4) store.add(listOf(listOf(Offset(0f, 0f), Offset(i.toFloat(), 1f))))
            val all = store.load()
            assertEquals(SavedSignatures.MAX, all.size)
            assertEquals(Offset(1f, 0.25f), all[0][0][1])
            store.remove(0)
            assertEquals(2, store.load().size)
        } finally {
            file.delete()
        }
    }
}
