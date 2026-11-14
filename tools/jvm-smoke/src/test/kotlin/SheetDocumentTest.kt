import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.nio.charset.Charset
import java.util.zip.ZipEntry
import java.util.zip.ZipInputStream
import java.util.zip.ZipOutputStream
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue
import org.mobileofficefree.engine.MoException
import org.mobileofficefree.engine.SheetDocument
import org.mobileofficefree.engine.SheetLoss
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
                // Read whole: nothing cut, so saving over the file is allowed.
                assertTrue(back.truncatedSheets().isEmpty())
                assertEquals(null, back.sharedStringsKept())
                assertTrue(back.unreadSheets().isEmpty())
                assertTrue(back.skippedParts().isEmpty())
                assertTrue(back.losses().isEmpty())
                assertTrue(!back.openedIncomplete())
            }
        }
    }

    @Test
    fun tellsWhatSavingBackWouldNotKeep() {
        val bytes = SheetDocument.blank().use { doc ->
            doc.execute("cell.set", """{"cell":"A1","input":"Gelir"}""")
            doc.save("senaryo.xlsx")
        }
        // A what-if scenario: GridCraft neither reads nor writes it back.
        val withScenario = rezip(bytes, "xl/worksheets/sheet1.xml") {
            it.replace("</sheetData>", "</sheetData><scenarios><scenario name=\"İyi\" count=\"1\"><inputCells r=\"A1\" val=\"5\"/></scenario></scenarios>")
        }
        SheetDocument.open("senaryo.xlsx", withScenario, null).use { doc ->
            assertEquals(listOf(SheetLoss.OTHER), doc.losses())
            assertTrue(doc.openedIncomplete())
        }
    }

    /** [bytes] (a zip) with the text of entry [name] changed by [edit]. */
    private fun rezip(bytes: ByteArray, name: String, edit: (String) -> String): ByteArray {
        val out = ByteArrayOutputStream()
        var found = false
        ZipInputStream(ByteArrayInputStream(bytes)).use { zin ->
            ZipOutputStream(out).use { zout ->
                while (true) {
                    val entry = zin.nextEntry ?: break
                    var data = zin.readBytes()
                    if (entry.name == name) {
                        data = edit(data.toString(Charsets.UTF_8)).toByteArray(Charsets.UTF_8)
                        found = true
                    }
                    zout.putNextEntry(ZipEntry(entry.name))
                    zout.write(data)
                    zout.closeEntry()
                }
            }
        }
        assertTrue(found, "no $name")
        return out.toByteArray()
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
