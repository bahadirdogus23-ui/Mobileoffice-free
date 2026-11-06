import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue
import org.mobileofficefree.engine.MoException
import org.mobileofficefree.engine.SheetDocument
import org.mobileofficefree.engine.SlidesDocument
import org.mobileofficefree.engine.WordDocument
import org.mobileofficefree.engine.blankDocument

class BlankDocumentTest {
    @Test
    fun newDocumentsOpenInTheirEngines() {
        WordDocument.open("Yeni.docx", blankDocument("docx"), null).use { assertTrue(it.pageCount() >= 1u) }
        SheetDocument.open("Yeni.xlsx", blankDocument("xlsx"), null).use { assertEquals(1, it.sheetNames().size) }
        SlidesDocument.open(blankDocument("pptx")).use { assertEquals(1u, it.slideCount()) }
        assertFailsWith<MoException.Save> { blankDocument("exe") }
    }
}
