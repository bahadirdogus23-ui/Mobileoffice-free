import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue
import org.mobileofficefree.engine.MoException
import org.mobileofficefree.engine.WordDocument

class WordDocumentTest {
    @Test
    fun rendersAPageAsPremultipliedRgba() {
        val md = "# Dilekçe\n\nİstanbul Şişli Belediyesi'ne; ğüşıöç ĞÜŞİÖÇ.\n".toByteArray()
        WordDocument.open("dilekce.md", md, null).use { doc ->
            assertEquals(1u, doc.pageCount())
            val size = doc.pageSize(0u)
            val r = doc.renderPage(0u, 0.5f)
            assertEquals(Math.ceil(size.width * 0.5).toUInt(), r.width)
            assertEquals((r.width * r.height * 4u).toInt(), r.pixels.size)
            val dark = r.pixels.toList().chunked(4).count { (it[0].toInt() and 0xff) < 128 }
            assertTrue(dark > 50, "text drawn: $dark dark pixels")
            assertFailsWith<MoException.NotFound> { doc.renderPage(3u, 1f) }
        }
    }
}
