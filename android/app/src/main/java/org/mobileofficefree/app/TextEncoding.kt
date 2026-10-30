package org.mobileofficefree.app

import java.nio.ByteBuffer
import java.nio.CharBuffer
import java.nio.charset.CharacterCodingException
import java.nio.charset.Charset
import java.nio.charset.CodingErrorAction

/** A text file encoding the app reads and writes; [bom] is written before the text. */
enum class TextEncoding(val label: String, private val charset: String, private val bom: ByteArray) {
    UTF8("UTF-8", "UTF-8", byteArrayOf()),
    UTF8_BOM("UTF-8 (BOM)", "UTF-8", byteArrayOf(0xEF.toByte(), 0xBB.toByte(), 0xBF.toByte())),
    UTF16LE("UTF-16", "UTF-16LE", byteArrayOf(0xFF.toByte(), 0xFE.toByte())),
    UTF16BE("UTF-16 BE", "UTF-16BE", byteArrayOf(0xFE.toByte(), 0xFF.toByte())),
    WINDOWS_1254("Windows-1254", "windows-1254", byteArrayOf()),
    ISO_8859_9("ISO-8859-9", "ISO-8859-9", byteArrayOf());

    /**
     * [text] in this encoding. A letter the encoding has no code for (an emoji in Windows-1254)
     * is an error, never a silent "?".
     */
    fun encode(text: String): ByteArray {
        val encoder = Charset.forName(charset).newEncoder()
            .onMalformedInput(CodingErrorAction.REPORT)
            .onUnmappableCharacter(CodingErrorAction.REPORT)
        val out = try {
            encoder.encode(CharBuffer.wrap(text))
        } catch (_: CharacterCodingException) {
            val bad = text.firstOrNull { !Charset.forName(charset).newEncoder().canEncode(it) }
            throw UnencodableText(label, bad)
        }
        return bom + ByteArray(out.remaining()).also { out.get(it) }
    }

    companion object {
        /** BOMs first; then strict UTF-8; otherwise Turkish Windows-1254, which most Turkish non-UTF-8 text is. */
        fun detect(bytes: ByteArray): Pair<String, TextEncoding> {
            fun starts(vararg b: Int) = bytes.size >= b.size && b.indices.all { (bytes[it].toInt() and 0xFF) == b[it] }
            return when {
                starts(0xEF, 0xBB, 0xBF) -> String(bytes, 3, bytes.size - 3, Charsets.UTF_8) to UTF8_BOM
                starts(0xFF, 0xFE) -> String(bytes, 2, bytes.size - 2, Charsets.UTF_16LE) to UTF16LE
                starts(0xFE, 0xFF) -> String(bytes, 2, bytes.size - 2, Charsets.UTF_16BE) to UTF16BE
                else -> try {
                    val strict = Charsets.UTF_8.newDecoder()
                        .onMalformedInput(CodingErrorAction.REPORT)
                        .onUnmappableCharacter(CodingErrorAction.REPORT)
                    strict.decode(ByteBuffer.wrap(bytes)).toString() to UTF8
                } catch (_: CharacterCodingException) {
                    String(bytes, Charset.forName("windows-1254")) to WINDOWS_1254
                }
            }
        }
    }
}

/** The text has a letter ([char]) that [encoding] cannot store. */
class UnencodableText(val encoding: String, val char: Char?) : Exception("“$char” cannot be written in $encoding")

/** Lines joined with Windows line ends when [crlf]; the editor works with "\n" only. */
fun withLineEnds(text: String, crlf: Boolean): String = if (crlf) text.replace("\r\n", "\n").replace("\n", "\r\n") else text

/**
 * Turkish text that went through the wrong code page, put right ("Turkish character recovery"):
 * - UTF-8 read as Latin-1/Windows-1252: "Ã§ok gÃ¼zel" → "çok güzel";
 * - Windows-1254 read as Windows-1252: "ÝSTANBUL, þeker, ðüzel" → "İSTANBUL, şeker, güzel".
 * Returns null when the text shows neither.
 */
fun repairTurkish(text: String): String? {
    // UTF-8 bytes seen one by one: a lead byte (Ã Ä Å …) followed by a continuation (€-¿).
    if (Regex("[Â-Å][\u0080-¿‘-›€ŒœŠšŸŽžƒˆ˜]").containsMatchIn(text)) {
        val bytes = runCatching { text.toByteArray(charset("windows-1252")) }.getOrNull()
        val fixed = bytes?.let { b -> runCatching { TextEncoding.detect(b) }.getOrNull()?.takeIf { it.second == TextEncoding.UTF8 }?.first }
        if (fixed != null && fixed != text) return fixed
    }
    // The six letters Windows-1254 has where Windows-1252 has Icelandic ones.
    val map = mapOf('Ð' to 'Ğ', 'ð' to 'ğ', 'Ý' to 'İ', 'ý' to 'ı', 'Þ' to 'Ş', 'þ' to 'ş')
    if (text.none { it in map }) return null
    return String(CharArray(text.length) { map[text[it]] ?: text[it] })
}
