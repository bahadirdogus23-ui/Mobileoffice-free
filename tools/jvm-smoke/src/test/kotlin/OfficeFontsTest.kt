import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import org.mobileofficefree.engine.SlideStyle
import org.mobileofficefree.engine.SlidesDocument
import org.mobileofficefree.engine.blankDocument
import org.mobileofficefree.engine.registerSlideFonts
import org.mobileofficefree.engine.registerWordFonts

/** The Office stand-in fonts the app ships as assets, handed over as `OfficeFonts` does it. */
class OfficeFontsTest {
    private val assets: List<ByteArray> =
        File("../../android/app/src/main/assets/fonts").listFiles { f -> f.name.endsWith(".ttf") }!!.sortedBy { it.name }.map { it.readBytes() }

    @Test
    fun theEnginesTakeTheBundledFontsOnce() {
        assertEquals(16, assets.size)
        // A desktop with the same fonts installed may have loaded some faces already: at most 16.
        assertTrue(registerSlideFonts(assets) <= 16u)
        assertEquals(0u, registerSlideFonts(assets), "calling again adds nothing")
        assertTrue(registerWordFonts(assets) <= 16u)
        assertEquals(0u, registerWordFonts(assets))
        // Files of another family are ignored.
        assertEquals(0u, registerSlideFonts(listOf("yazı tipi değil".toByteArray())))

        // A text box in Calibri draws, and exports, in Carlito.
        SlidesDocument.open(blankDocument("pptx")).use { d ->
            val id = d.addTextBox(0u, "Çağrı Işık, Şişli Göğüs İstanbul")
            d.styleObject(0u, id, SlideStyle.Font("Calibri"))
            val pdf = String(d.exportPdf(), Charsets.ISO_8859_1)
            assertTrue("Carlito" in pdf, "the PDF embeds Carlito")
        }
    }
}
