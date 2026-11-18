import java.io.ByteArrayOutputStream
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import org.mobileofficefree.engine.PdfDocument
import org.mobileofficefree.engine.registerPdfFonts

/**
 * A PDF in non-embedded Helvetica whose /Differences name Ğ ğ İ Ş ş by glyph (old Turkish PDFs):
 * Android's renderer lacks those glyphs, so the viewer shows `displayCopy`, which embeds a stand-in.
 */
class StandInFontsTest {
    private fun base14Turkish(): ByteArray {
        val content = byteArrayOf(*"BT /F1 24 Tf 40 760 Td (Tutana".toByteArray(), 0x81.toByte(), 0x83.toByte(), ' '.code.toByte(), 0x84.toByte(), *"ube) Tj ET".toByteArray())
        val objects = listOf(
            "<< /Type /Catalog /Pages 2 0 R >>".toByteArray(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".toByteArray(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".toByteArray(),
            ("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding << /BaseEncoding /WinAnsiEncoding " +
                "/Differences [128 /Gbreve /gbreve /Idotaccent /dotlessi /Scedilla /scedilla] >> >>").toByteArray(),
            "<< /Length ${content.size} >>\nstream\n".toByteArray() + content + "\nendstream".toByteArray(),
        )
        val out = ByteArrayOutputStream()
        out.write("%PDF-1.4\n".toByteArray())
        val offsets = objects.mapIndexed { i, body ->
            val at = out.size()
            out.write("${i + 1} 0 obj\n".toByteArray())
            out.write(body)
            out.write("\nendobj\n".toByteArray())
            at
        }
        val xref = out.size()
        out.write("xref\n0 ${objects.size + 1}\n0000000000 65535 f \n".toByteArray())
        offsets.forEach { out.write("%010d 00000 n \n".format(it).toByteArray()) }
        out.write("trailer << /Size ${objects.size + 1} /Root 1 0 R >>\nstartxref\n$xref\n%%EOF\n".toByteArray())
        return out.toByteArray()
    }

    @Test
    fun turkishLettersOfANonEmbeddedHelveticaGetAStandIn() {
        val liberation = File("../../android/app/src/main/assets/fonts").listFiles { f -> f.name.startsWith("Liberation") }!!
            .sortedBy { it.name }.map { it.readBytes() }
        assertTrue(registerPdfFonts(liberation) <= 8u)
        assertEquals(0u, registerPdfFonts(liberation), "calling again adds nothing")
        PdfDocument.open(base14Turkish(), null).use { doc ->
            assertTrue(doc.needsStandInFonts())
            PdfDocument.open(doc.displayCopy(), null).use { copy ->
                assertEquals(false, copy.needsStandInFonts())
                assertTrue(copy.pageText(0u).contains("Tutanağı Şube"), copy.pageText(0u))
            }
            // The document itself keeps its font.
            PdfDocument.open(doc.save(), null).use { assertTrue(it.needsStandInFonts()) }
        }
    }
}
