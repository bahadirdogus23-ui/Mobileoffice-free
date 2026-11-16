package org.mobileofficefree.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import kotlin.random.Random

class OcrPrepTest {
    @Test
    fun picturesAreScaledSoTheirLongestSideSuitsTesseract() {
        // The owner's photographed sign: 2480×5367 comes down to 2600 px tall.
        val f = OcrImage.scaleFactor(2480, 5367)
        assertEquals(OcrImage.TARGET_LONG_SIDE, Math.round(5367 * f).toInt())
        assertEquals(1.0, OcrImage.scaleFactor(2480, 2480), 0.0)
        assertEquals(1.0, OcrImage.scaleFactor(3000, 1000), 0.0)
        // A small picture is enlarged, but not beyond 2.5×.
        assertEquals(2.5, OcrImage.scaleFactor(640, 480), 1e-9)
        assertEquals(2600.0 / 1500, OcrImage.scaleFactor(1500, 1000), 1e-9)
        assertEquals(1.0, OcrImage.scaleFactor(0, 0), 0.0)
    }

    @Test
    fun otsuSplitsTwoTonesBetweenThem() {
        val rnd = Random(7)
        val gray = IntArray(10_000) { i -> if (i % 3 == 0) 40 + rnd.nextInt(-10, 11) else 200 + rnd.nextInt(-15, 16) }
        val t = OcrImage.otsu(gray)
        assertTrue("threshold $t", t in 50..184)
        assertTrue(gray.filter { it <= t }.all { it <= 50 })
    }

    @Test
    fun contrastIsStretchedToTheFullRange() {
        val gray = IntArray(1000) { i -> 100 + i % 50 }
        OcrImage.stretchContrast(gray)
        assertTrue(gray.min() <= 5)
        assertTrue(gray.max() >= 250)
        // A flat picture stays as it is.
        val flat = IntArray(100) { 128 }
        OcrImage.stretchContrast(flat)
        assertTrue(flat.all { it == 128 })
    }

    /**
     * A white sign lit from the left (255 fading to 150 on the right) on a darker, speckled wall,
     * with dark letters: one thin, one thick. Global Otsu alone would blacken the shadowed side;
     * the letters must come out solid black and the sign and wall white.
     */
    @Test
    fun unevenlyLitSignBecomesBlackTextOnWhite() {
        val w = 800
        val h = 600
        val rnd = Random(3)
        val gray = IntArray(w * h)
        fun onSign(x: Int, y: Int) = x in 100 until 700 && y in 100 until 500
        fun thinLetter(x: Int, y: Int) = x in 150 until 158 && y in 200 until 300
        fun thickLetter(x: Int, y: Int) = x in 560 until 620 && y in 200 until 400
        for (y in 0 until h) {
            for (x in 0 until w) {
                val light = 255 - 105 * x / w
                gray[y * w + x] = when {
                    !onSign(x, y) -> (light * 55 / 100 + rnd.nextInt(-6, 7)).coerceIn(0, 255)
                    thinLetter(x, y) || thickLetter(x, y) -> light * 25 / 100
                    else -> light
                }
            }
        }
        val bw = OcrImage.binarize(gray, w, h)
        assertTrue(bw.all { it == 0 || it == 255 })
        var letterInk = 0
        var letters = 0
        var stray = 0
        var signPixels = 0
        for (y in 0 until h) {
            for (x in 0 until w) {
                val ink = bw[y * w + x] == 0
                if (thinLetter(x, y) || thickLetter(x, y)) {
                    letters++
                    if (ink) letterInk++
                } else if (onSign(x, y)) {
                    signPixels++
                    if (ink) stray++
                }
            }
        }
        assertEquals("every letter pixel is ink, thick strokes solid", letters, letterInk)
        assertTrue("sign background stays white ($stray stray)", stray * 1000 < signPixels)
        // The middle of the thick letter, far from its edges, is ink too.
        assertEquals(0, bw[300 * w + 590])
        // The shadowed right side of the sign is white, not black.
        assertEquals(255, bw[450 * w + 680])
    }

    @Test
    fun preparedPixelsAreOpaqueBlackOrWhite() {
        val argb = IntArray(64 * 64) { i -> if ((i % 64) in 20..24) 0xFF202020.toInt() else 0xFFE0E0E0.toInt() }
        val out = OcrImage.prepare(argb, 64, 64)
        assertTrue(out.all { it == 0xFF000000.toInt() || it == 0xFFFFFFFF.toInt() })
        assertEquals(0xFF000000.toInt(), out[10 * 64 + 22])
        assertEquals(0xFFFFFFFF.toInt(), out[10 * 64 + 50])
    }

    private fun line(vararg words: Pair<String, Float>, newBlock: Boolean = false) =
        OcrReadLine(words.map { (t, c) -> OcrReadWord(t, c) }, newBlock)

    /** Lines the owner got from a photo of the "BİNA KİMLİK SERTİFİKASI" sign. */
    @Test
    fun noiseLinesAroundASignAreLeftOut() {
        val noise = listOf(
            line("i," to 41f, "." to 30f),
            line("RAMA" to 35f, "a" to 21f, "ON" to 30f),
            line("MWE" to 30f, "İŞLEME" to 45f, "PE" to 25f, "İLİM" to 38f),
            // Fragments are noise even when Tesseract is half sure of them.
            line("RAMA" to 55f, "a" to 55f, "ON" to 55f),
            line("—" to 90f, "|" to 90f, "~" to 80f),
            line("|" to 95f, "l" to 95f),
        )
        noise.forEach { assertFalse("kept noise ${it.words.map { w -> w.text }}", OcrLines.keep(it)) }
        val text = listOf(
            line("Bina" to 91f, "Kimlik" to 88f, "Sistemine" to 85f, "ait" to 92f, "her" to 90f, "türlü" to 86f, "bilgi" to 89f),
            line("BİNA" to 93f, "KİMLİK" to 90f, "SERTİFİKASI" to 87f),
            // One doubtful word does not throw away a line Tesseract is otherwise sure of.
            line("Ada:" to 88f, "1234" to 92f, "Parsel:" to 85f, "5x" to 12f),
            line("Kat" to 62f, "3" to 70f),
        )
        text.forEach { assertTrue("dropped ${it.words.map { w -> w.text }}", OcrLines.keep(it)) }
    }

    @Test
    fun keptLinesKeepTheirOrderAndParagraphs() {
        val lines = listOf(
            line("BİNA" to 93f, "KİMLİK" to 90f, "SERTİFİKASI" to 87f, newBlock = true),
            line("i," to 41f, "." to 30f),
            line("RAMA" to 35f, "a" to 21f, "ON" to 30f, newBlock = true),
            line("Bina" to 91f, "Kimlik" to 88f, "Sistemine" to 85f),
            line("ait" to 92f, "her" to 90f, "türlü" to 86f, "bilgi" to 89f),
        )
        assertEquals("BİNA KİMLİK SERTİFİKASI\n\nBina Kimlik Sistemine\nait her türlü bilgi", OcrLines.text(lines))
        assertEquals(3, OcrLines.kept(lines).size)
        assertEquals("", OcrLines.text(listOf(line("." to 10f))))
    }
}
