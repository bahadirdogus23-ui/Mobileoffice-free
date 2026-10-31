package org.mobileofficefree.app

import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import org.junit.Assert.assertEquals
import org.junit.Test

class KeyboardEditTest {
    /** The document's text, edited the way the engine would be. */
    private val doc = StringBuilder("Önce ")
    private var field = EMPTY_INPUT

    private fun key(text: String, composing: TextRange? = null) {
        field = keyboardEdit(field, TextFieldValue(text, TextRange(text.length), composing)) { delete, typed ->
            repeat(delete) { doc.deleteAt(doc.length - 1) }
            doc.append(typed)
        }
    }

    @Test
    fun lettersAppearAsTheyAreTyped() {
        key(SENTINEL + "ğ")
        assertEquals("Önce ğ", doc.toString())
        assertEquals(SENTINEL + "ğ", field.text)
    }

    @Test
    fun aWordBeingComposedIsShownAtOnceAndTypedOnlyOnce() {
        key(SENTINEL + "M", TextRange(1, 2))
        key(SENTINEL + "Me", TextRange(1, 3))
        key(SENTINEL + "Merh", TextRange(1, 5))
        assertEquals("Önce Merh", doc.toString())
        key(SENTINEL + "Merhaba ")
        assertEquals("Önce Merhaba ", doc.toString())
        key(SENTINEL + "Merhaba d", TextRange(9, 10))
        assertEquals("Önce Merhaba d", doc.toString())
    }

    @Test
    fun autocorrectionReplacesTheWord() {
        key(SENTINEL + "merhba", TextRange(1, 7))
        key(SENTINEL + "merhaba ")
        assertEquals("Önce merhaba ", doc.toString())
    }

    @Test
    fun deletingInTheFieldAndPastItAreBackspaces() {
        key(SENTINEL + "ab")
        key(SENTINEL + "a")
        assertEquals("Önce a", doc.toString())
        key(SENTINEL)
        key("")
        assertEquals("Önce", doc.toString())
        assertEquals(EMPTY_INPUT, field)
    }

    @Test
    fun enterArrivesAsANewLineAndStartsTheFieldAfresh() {
        key(SENTINEL + "a\n")
        assertEquals("Önce a\n", doc.toString())
        assertEquals(EMPTY_INPUT, field)
    }
}
