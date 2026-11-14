import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue
import org.mobileofficefree.engine.MoException
import org.mobileofficefree.engine.PdfDocument
import org.mobileofficefree.engine.NamedFile
import org.mobileofficefree.engine.Pen
import org.mobileofficefree.engine.imagesToPdf
import org.mobileofficefree.engine.SensitiveKind
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
    fun tellsWhatOpeningRepaired() {
        val sound = pdf()
        PdfDocument.open(sound, null).use { assertEquals(emptyList<String>(), it.openNotes()) }
        // The startxref offset zeroed (same length): the index has to be rebuilt.
        val text = String(sound, Charsets.ISO_8859_1)
        val at = text.lastIndexOf("startxref") + "startxref".length
        val start = at + text.substring(at).takeWhile { it.isWhitespace() }.length
        val end = start + text.substring(start).takeWhile { it.isDigit() }.length
        assertTrue(end > start)
        val broken = sound.copyOf().also { for (i in start until end) it[i] = '0'.code.toByte() }
        PdfDocument.open(broken, null).use { doc ->
            assertTrue(doc.openNotes().any { it.contains("rebuilt") }, doc.openNotes().toString())
            assertEquals(1u, doc.pageCount())
            PdfDocument.open(doc.save(), null).use { assertEquals(emptyList<String>(), it.openNotes()) }
        }
    }

    @Test
    fun refusesAFileWithoutReadablePages() {
        val sound = pdf()
        // The catalog's /Pages key renamed (same length): the engine finds no page tree.
        val at = String(sound, Charsets.ISO_8859_1).indexOf("/Pages")
        assertTrue(at >= 0)
        val damaged = sound.copyOf().also { it[at + 1] = 'X'.code.toByte() }
        assertFailsWith<MoException.NoPages> { PdfDocument.open(damaged, null) }
        // Combining refuses it too, leaving the document as it was.
        PdfDocument.open(sound, null).use { doc ->
            assertFailsWith<MoException.NoPages> { doc.append(damaged, null) }
            assertEquals(1u, doc.pageCount())
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

    @Test
    fun passwords() {
        val locked = PdfDocument.open(pdf(), null).use { doc ->
            doc.protect("şifre", ByteArray(32) { it.toByte() })
            doc.save()
        }
        assertFailsWith<MoException.NeedsPassword> { PdfDocument.open(locked, null) }
        assertFailsWith<MoException.WrongPassword> { PdfDocument.open(locked, "yanlış") }
        PdfDocument.open(locked, "şifre").use { doc ->
            assertEquals(true, doc.isProtected())
            assertEquals(false, doc.isRestricted())
            PdfDocument.open(doc.displayCopy(), null).use { assertEquals(false, it.isProtected()) }
        }
    }

    @Test
    fun compresses() {
        PdfDocument.open(pdf(), null).use { doc ->
            PdfDocument.open(doc.compress(true), null).use { assertEquals(1u, it.pageCount()) }
        }
    }

    @Test
    fun aFormlessPdfHasNoFields() {
        PdfDocument.open(pdf(), null).use { doc ->
            assertEquals(0, doc.fields().size)
            assertFailsWith<MoException.Command> { doc.setText("ad", "Şule") }
        }
    }

    @Test
    fun drawsAndUndoes() {
        PdfDocument.open(pdf(), null).use { doc ->
            doc.draw(0u, listOf(listOf(0.1, 0.5, 0.9, 0.5)), Pen(1.0, 0.9, 0.0, 12.0, 0.4))
            doc.note(0u, 0.5, 0.3, "Not: ığüşöç")
            assertEquals(true, doc.undoAnnotation())
            assertEquals(true, doc.undoAnnotation())
            assertEquals(false, doc.undoAnnotation())
        }
    }

    @Test
    fun searchesAndBlacksOut() {
        val bytes = SheetDocument.blank().use { sheet ->
            sheet.execute("cell.set", """{"cell":"A1","input":"IBAN TR33 0006 1005 1978 6457 8413 26"}""")
            sheet.execute("cell.set", """{"cell":"A2","input":"İSTANBUL"}""")
            sheet.exportPdf(true, null, false)
        }
        PdfDocument.open(bytes, null).use { doc ->
            assertEquals(1, doc.findText("istanbul", 10u).size)
            assertEquals(1, doc.markRedactions(listOf(SensitiveKind.IBAN), emptyList()).size)
            assertEquals(1u, doc.redactionMarks())
            assertEquals(1u, doc.applyRedactions())
            PdfDocument.open(doc.save(), null).use { assertEquals(false, it.pageText(0u).contains("TR33")) }
        }
    }

    @Test
    fun picturesBecomeAPdf() {
        // A 1×1 PNG.
        val png = java.util.Base64.getDecoder().decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4z8AAAAMBAQDJ/pLvAAAAAElFTkSuQmCC")
        val pdf = imagesToPdf(listOf(NamedFile("a.png", png), NamedFile("b.png", png)), true)
        PdfDocument.open(pdf, null).use { assertEquals(2u, it.pageCount()) }
        assertFailsWith<MoException.Command> { imagesToPdf(emptyList(), true) }
    }
}
