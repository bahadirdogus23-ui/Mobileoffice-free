import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import org.mobileofficefree.engine.MoException
import org.mobileofficefree.engine.PdfDocument
import org.mobileofficefree.engine.SheetDocument

class PdfDocumentTest {
    /** A small PDF made by the spreadsheet engine: one page per sheet. */
    private fun pdf(): ByteArray = SheetDocument.blank().use { it.exportPdf(true, null, false) }

    @Test
    fun pageToolsRoundTrip() {
        PdfDocument.open(pdf(), null).use { doc ->
            doc.append(pdf(), null)
            assertEquals(2u, doc.pageCount())
            doc.rotate(listOf(1u), 90)
            doc.movePages(listOf(1u), 0u)
            assertFailsWith<MoException.Command> { doc.delete(listOf(0u, 1u)) }
            val part = doc.extract(listOf(0u))
            PdfDocument.open(part, null).use { assertEquals(1u, it.pageCount()) }
            PdfDocument.open(doc.save(), null).use { assertEquals(2u, it.pageCount()) }
        }
    }

    @Test
    fun signsAPage() {
        PdfDocument.open(pdf(), null).use { doc ->
            doc.sign(0u, listOf(listOf(0.6, 0.8, 0.7, 0.85, 0.8, 0.8)))
            assertFailsWith<MoException.Command> { doc.sign(0u, listOf(emptyList())) }
            PdfDocument.open(doc.save(), null).use { assertEquals(1u, it.pageCount()) }
        }
    }
}
