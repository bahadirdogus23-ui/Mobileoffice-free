import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import org.mobileofficefree.engine.CadDocument

class CadDocumentTest {
    @Test
    fun dxfRendersAView() {
        val dxf = File("../../rust/mo-cad/tests/data/apartment.dxf").readBytes()
        CadDocument.open("apartment.dxf", dxf).use { doc ->
            assertEquals("Model", doc.spaceNames().first())
            assertTrue(doc.layers().isNotEmpty())
            val e = doc.extents(0u)
            val scale = 300.0 / (e.maxX - e.minX)
            val r = doc.render(0u, (e.minX + e.maxX) / 2, (e.minY + e.maxY) / 2, scale, 300u, 200u, false)
            assertEquals(300u * 200u * 4u, r.pixels.size.toUInt())
            val drawn = r.pixels.toList().chunked(4).count { (it[0].toInt() and 0xff) < 250 }
            assertTrue(drawn > 500, "drawn pixels: $drawn")
        }
    }
}
