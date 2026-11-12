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

    @Test
    fun aTappedCellGoesIntoTheFormulaAndATapAfterReplacesIt() {
        val (v1, r1) = insertReference(at("=TOPLA("), "B4", null)
        assertEquals("=TOPLA(B4", v1.text)
        val (v2, r2) = insertReference(v1, "C7", r1)
        assertEquals("=TOPLA(C7", v2.text)
        assertEquals(TextRange(7, 9), r2)
        // After typing something else, a tap adds a new reference.
        val typed = typeAt(v2, ";")
        assertEquals("=TOPLA(C7;D1", insertReference(typed, "D1", r2).first.text)
        assertEquals(true, pointing(at("=")))
        assertEquals(false, pointing(at("12")))
    }

    @Test
    fun findsTheFunctionAroundTheCursor() {
        assertEquals("EĞER", functionAtCursor(at("=eğer(A1>2;")))
        assertEquals("TOPLA", functionAtCursor(at("=EĞER(TOPLA(A1:A3")))
        assertEquals("EĞER", functionAtCursor(at("=EĞER(TOPLA(A1:A3);")))
        assertEquals(null, functionAtCursor(at("=A1+2")))
        assertEquals("EĞER", functionAtCursor(at("=EĞER(A1=\"(\";")))
    }

    @Test
    fun quotedTextNeverOffersFunctionsOrInsertsCellReferences() {
        for (text in listOf("=\"bu top", "=EĞER(A1;\"bu top", "=\"bir \"\"alıntı\"\" top")) {
            assertEquals(text, emptyList<String>(), suggestions(at(text), names))
            assertEquals(text, false, pointing(at(text)))
            assertEquals(text, at(text), applySuggestion(at(text), "TOPLA"))
        }
        assertEquals(listOf("TOPLA", "TOPLAÇARPIM"), suggestions(at("=\"metin\"&top"), names))
        assertEquals(true, pointing(at("=\"metin\"&")))
    }

    @Test
    fun quotedSheetNamesAreNotFunctionNamesOrPointingLocations() {
        assertEquals(emptyList<String>(), suggestions(at("='Bütçe top"), names))
        assertEquals(false, pointing(at("='Ali''nin sayfası")))
        assertEquals(true, pointing(at("='Ali''nin sayfası'!")))
    }
}
