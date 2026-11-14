import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import org.mobileofficefree.engine.MoException
import org.mobileofficefree.engine.decodeTextFile
import org.mobileofficefree.engine.encodeTextFile

class TextFileTest {
    private val turkish = "Şişli'de ığdır çağrı İĞÜŞÖÇ\r\n“alıntı” – tire\r\n"

    @Test
    fun textFilesComeBackInTheirOwnEncoding() {
        val utf16le = turkish.toByteArray(Charsets.UTF_16LE)
        val cases = listOf(
            Triple(turkish.toByteArray(charset("windows-1254")), "windows-1254", false),
            Triple(turkish.toByteArray(Charsets.UTF_8), "UTF-8", false),
            Triple(byteArrayOf(0xEF.toByte(), 0xBB.toByte(), 0xBF.toByte()) + turkish.toByteArray(Charsets.UTF_8), "UTF-8", true),
            Triple(utf16le, "UTF-16LE", false),
            Triple(byteArrayOf(0xFF.toByte(), 0xFE.toByte()) + utf16le, "UTF-16LE", true),
            Triple(byteArrayOf(0xFE.toByte(), 0xFF.toByte()) + turkish.toByteArray(Charsets.UTF_16BE), "UTF-16BE", true),
        )
        for ((bytes, name, bom) in cases) {
            val d = decodeTextFile(bytes, true)
            assertEquals(turkish, d.text, name)
            assertEquals(name, d.encoding)
            assertEquals(bom, d.bom, name)
            assertTrue(d.exact, name)
            assertContentEquals(bytes, encodeTextFile(d.text, d.encoding, d.bom), "$name bom=$bom")
        }
        // Russian Windows-1251 is not taken for Turkish.
        val ru = "Привет, как дела? Хорошо, спасибо.".toByteArray(charset("windows-1251"))
        assertEquals("windows-1251", decodeTextFile(ru, true).encoding)
    }

    @Test
    fun theStartOfALargeFileDropsACutLetter() {
        val bytes = "aaaş".toByteArray()
        val d = decodeTextFile(bytes.copyOf(bytes.size - 1), false)
        assertEquals("aaa", d.text)
        assertEquals("UTF-8", d.encoding)
    }

    @Test
    fun lettersTheEncodingLacksAreRefused() {
        val e = assertFailsWith<MoException.Unencodable> { encodeTextFile("Merhaba 😀", "windows-1254", false) }
        assertEquals("😀", e.message)
        assertFailsWith<MoException.Save> { encodeTextFile("x", "no-such-encoding", false) }
        // A lone surrogate decodes to "�": saving it back would change the file.
        val lone = byteArrayOf(0xFF.toByte(), 0xFE.toByte(), 0x61, 0, 0, 0xD8.toByte(), 0x62, 0)
        assertFalse(decodeTextFile(lone, true).exact)
    }
}
