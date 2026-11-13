import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue
import org.mobileofficefree.engine.MoException
import org.mobileofficefree.engine.WordDocument
import org.mobileofficefree.engine.WordLoss

class WordDocumentTest {
    @Test
    fun rendersAPageAsPremultipliedRgba() {
        val md = "# Dilekçe\n\nİstanbul Şişli Belediyesi'ne; ğüşıöç ĞÜŞİÖÇ.\n".toByteArray()
        WordDocument.open("dilekce.md", md, null).use { doc ->
            assertEquals(1u, doc.pageCount())
            val size = doc.pageSize(0u)
            val r = doc.renderPage(0u, 0.5f, false)
            assertEquals(Math.ceil(size.width * 0.5).toUInt(), r.width)
            assertEquals((r.width * r.height * 4u).toInt(), r.pixels.size)
            val dark = r.pixels.toList().chunked(4).count { (it[0].toInt() and 0xff) < 128 }
            assertTrue(dark > 50, "text drawn: $dark dark pixels")
            assertFailsWith<MoException.NotFound> { doc.renderPage(3u, 1f, false) }
            val hits = doc.find("ISTANBUL")
            assertTrue(hits.isNotEmpty() && hits.all { it.page == 0u && it.width > 0f }, "$hits")
        }
    }

    @Test
    fun editsThroughCommands() {
        WordDocument.open("a.md", "Merhaba.\n".toByteArray(), null).use { doc ->
            val hit = doc.find("Merhaba").first()
            doc.tap(hit.page, hit.x + 0.5f, hit.y + hit.height / 2, false)
            doc.execute("text.insert", """{"text":"İyi günler, "}""")
            assertEquals(true, doc.isDirty())
            assertEquals(1, doc.find("iyi gunler").size)
            assertEquals(true, doc.caret() != null)
            doc.markSaved()
            assertEquals(false, doc.isDirty())
            assertFailsWith<MoException.Command> { doc.execute("no.such", "") }
        }
    }

    @Test
    fun tellsWhatOpeningLeftOut() {
        // Read whole: saving over the file is allowed.
        WordDocument.open("a.md", "Merhaba.\n".toByteArray(), null).use { doc ->
            assertTrue(doc.losses().isEmpty())
            assertTrue(!doc.openedIncomplete())
        }
        // An RTF footnote is not read: saving over the file would delete it.
        val rtf = "{\\rtf1\\ansi Metin{\\footnote\\pard Dipnot}\\par}".toByteArray()
        WordDocument.open("a.rtf", rtf, null).use { doc ->
            assertEquals(listOf(WordLoss.NOTES), doc.losses())
            assertTrue(doc.openedIncomplete())
        }
        // A web page's style sheet is dropped.
        WordDocument.open("a.html", "<style>p{}</style><p>x</p>".toByteArray(), null).use { doc ->
            assertEquals(listOf(WordLoss.WEB_CONTENT), doc.losses())
        }
    }
}
