package org.mobileofficefree.app

import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import org.junit.Assert.assertEquals
import org.junit.Test

class KeyboardEditTest {
    private val typed = mutableListOf<String>()
    private var backspaces = 0

    private fun edit(text: String, composing: TextRange? = null) =
        keyboardEdit(TextFieldValue(text, TextRange(text.length), composing), { typed += it }, { backspaces++ })

    @Test
    fun committedTextIsTypedAndTheFieldEmpties() {
        assertEquals(EMPTY_INPUT, edit(SENTINEL + "ğ"))
        assertEquals(listOf("ğ"), typed)
    }

    @Test
    fun aWordBeingComposedWaitsForTheKeyboard() {
        val shown = edit(SENTINEL + "Merh", TextRange(1, 5))
        assertEquals(SENTINEL + "Merh", shown.text)
        assertEquals(emptyList<String>(), typed)
        edit(SENTINEL + "Merhaba ")
        assertEquals(listOf("Merhaba "), typed)
    }

    @Test
    fun deletingTheStandInIsABackspace() {
        assertEquals(EMPTY_INPUT, edit(""))
        assertEquals(1, backspaces)
        edit("x")
        assertEquals(listOf("x"), typed)
    }

    @Test
    fun enterArrivesAsANewLine() {
        edit(SENTINEL + "\n")
        assertEquals(listOf("\n"), typed)
    }
}
