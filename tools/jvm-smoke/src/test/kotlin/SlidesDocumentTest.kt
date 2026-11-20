import java.io.File
import java.util.zip.ZipInputStream
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue
import org.mobileofficefree.engine.MoException
import org.mobileofficefree.engine.SlideObjectKind
import org.mobileofficefree.engine.SlidesDocument

class SlidesDocumentTest {
    @Test
    fun notAPresentationIsAnOpenError() {
        assertFailsWith<MoException.Open> { SlidesDocument.open("sunum değil".toByteArray()) }
    }

    /** "Tablo stili": the picker's choices come with theme colours, and the chosen one is saved by Office's GUID. */
    @Test
    fun aTableStyleIsOfferedAppliedAndSaved() {
        val pptx = File("../../rust/mo-deck/testdata/grouped-text.pptx").readBytes()
        SlidesDocument.open(pptx).use { doc ->
            val styles = doc.tableStyles(0u)
            assertEquals(7, styles.size)
            assertEquals("medium2-accent3", styles[2].id)
            assertEquals(0x9BBB59u, styles[2].color)
            val table = doc.objects(0u).first { it.kind == SlideObjectKind.TABLE }
            doc.setTableStyle(0u, table.id, "medium2-accent3")
            assertFailsWith<MoException> { doc.setTableStyle(0u, table.id, "yok") }
            val slide = entry(doc.save("x.pptx"), "ppt/slides/slide1.xml")
            assertTrue("{F5AB1C69-6EDB-4FF4-983F-18BD219EF322}" in slide, slide)
        }
    }

    private fun entry(zip: ByteArray, name: String): String {
        ZipInputStream(zip.inputStream()).use { z ->
            while (true) {
                val e = z.nextEntry ?: error("no $name")
                if (e.name == name) return z.readBytes().decodeToString()
            }
        }
    }
}
