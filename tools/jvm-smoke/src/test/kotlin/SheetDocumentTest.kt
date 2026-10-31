import java.nio.charset.Charset
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue
import org.mobileofficefree.engine.MoException
import org.mobileofficefree.engine.SheetDocument
import org.mobileofficefree.engine.nativeVersion

class SheetDocumentTest {
    @Test
    fun editsThroughCommandsAndRoundTripsXlsx() {
        assertTrue(nativeVersion().isNotEmpty())
        SheetDocument.blank().use { doc ->
            doc.execute("cell.set", """{"cell":"A1","input":"Gider"}""")
            doc.execute("cell.set", """{"cell":"B1","input":"750"}""")
            doc.execute("cell.set", """{"cell":"B2","input":"=B1*4"}""")
            assertEquals(listOf("Gider", "750", "", "3000"), doc.cellTexts(0u, 0u, 0u, 2u, 2u))
            assertEquals("=B1*4", doc.editText(0u, 1u, 1u))
            val bytes = doc.save("butce.xlsx")
            SheetDocument.open("butce.xlsx", bytes, null).use { back ->
                assertEquals(2u, back.usedSize(0u).rows)
                assertEquals("3000", back.cellTexts(0u, 1u, 1u, 1u, 1u).single())
            }
        }
    }

    @Test
    fun turkishCsvSurvivesTheBoundary() {
        val bytes = "Şehir;İlçe\nİzmir;Karşıyaka\n".toByteArray(Charset.forName("windows-1254"))
        SheetDocument.open("liste.csv", bytes, null).use { doc ->
            assertEquals(listOf("Şehir", "İlçe", "İzmir", "Karşıyaka"), doc.cellTexts(0u, 0u, 0u, 2u, 2u))
        }
    }

    @Test
    fun engineErrorsBecomeKotlinExceptions() {
        assertFailsWith<MoException.Open> { SheetDocument.open("bozuk.xlsx", byteArrayOf(1, 2, 3), null) }
        SheetDocument.blank().use { doc ->
            assertFailsWith<MoException.Command> { doc.execute("no.such.command", "{}") }
            assertFailsWith<MoException.NotFound> { doc.usedSize(7u) }
        }
    }

    @Test
    fun turkishInputAndLooks() {
        SheetDocument.blank().use { doc ->
            doc.setTurkish(true)
            doc.setInput(0u, 0u, "1.250,5")
            doc.setInput(1u, 0u, "=TOPLA(A1;0,5)")
            assertEquals(listOf("1250,5", "1251"), doc.cellTexts(0u, 0u, 0u, 2u, 1u))
            assertEquals("=TOPLA(A1;0,5)", doc.editText(0u, 1u, 0u))
            doc.execute("home.bold", """{"range":"A1"}""")
            assertEquals(true, doc.cellLooks(0u, 0u, 0u, 1u, 1u)[0].bold)
        }
    }
}
