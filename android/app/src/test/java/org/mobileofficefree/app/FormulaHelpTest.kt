package org.mobileofficefree.app

import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import org.junit.Assert.assertEquals
import org.junit.Test

class FormulaHelpTest {
    private val names = listOf("BUL", "BUGÜN", "DÜŞEYARA", "EĞER", "EĞERHATA", "TOPLA", "TOPLAÇARPIM")

    private fun at(text: String) = TextFieldValue(text, TextRange(text.length))

    @Test
    fun suggestsNamesForWhatIsTyped() {
        assertEquals(listOf("TOPLA", "TOPLAÇARPIM"), suggestions(at("=top"), names))
        assertEquals(listOf("EĞER", "EĞERHATA"), suggestions(at("=A1+eğ"), names))
        assertEquals(listOf("BUL", "BUGÜN"), suggestions(at("=EĞER(bu"), names))
        assertEquals(emptyList<String>(), suggestions(at("top"), names))
        assertEquals(emptyList<String>(), suggestions(at("=\"top"), names))
    }

    @Test
    fun aSuggestionOpensItsBrackets() {
        val v = applySuggestion(at("=1+top"), "TOPLA")
        assertEquals("=1+TOPLA(", v.text)
        assertEquals(TextRange(9), v.selection)
        assertEquals("=TOPLA(A1;", typeAt(at("=TOPLA(A1"), ";").text)
    }
}
