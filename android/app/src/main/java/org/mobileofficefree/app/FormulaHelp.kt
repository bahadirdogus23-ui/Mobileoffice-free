package org.mobileofficefree.app

import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import java.util.Locale

/** Turkish upper case (i → İ, ı → I), as function names are written. */
private fun upperTr(s: String): String = s.uppercase(Locale.forLanguageTag("tr"))

/** The function name being typed just before the cursor in a formula, or null. */
fun nameBeforeCursor(value: TextFieldValue): String? {
    val text = value.text
    if (!text.startsWith("=")) return null
    val end = value.selection.start.coerceIn(0, text.length)
    var start = end
    while (start > 0 && (text[start - 1].isLetter() || text[start - 1] == '_')) start--
    // A name starts after an operator, a bracket or a separator, never inside a cell reference.
    if (start == end || (start > 0 && (text[start - 1].isLetterOrDigit() || text[start - 1] == '"'))) return null
    return text.substring(start, end)
}

/** Function names starting with what is typed, at most [limit]. */
fun suggestions(value: TextFieldValue, names: List<String>, limit: Int = 8): List<String> {
    val typed = nameBeforeCursor(value) ?: return emptyList()
    val up = upperTr(typed)
    return names.filter { it.startsWith(up) && it != up }.take(limit)
}

/** [value] with the name being typed replaced by `name(`, the cursor inside the brackets. */
fun applySuggestion(value: TextFieldValue, name: String): TextFieldValue {
    val typed = nameBeforeCursor(value) ?: return value
    val end = value.selection.start
    val start = end - typed.length
    val text = value.text.substring(0, start) + name + "(" + value.text.substring(end)
    return TextFieldValue(text, TextRange(start + name.length + 1))
}

/** [value] with [piece] typed at the cursor (replacing a selection). */
fun typeAt(value: TextFieldValue, piece: String): TextFieldValue {
    val a = value.selection.min.coerceIn(0, value.text.length)
    val b = value.selection.max.coerceIn(0, value.text.length)
    val text = value.text.substring(0, a) + piece + value.text.substring(b)
    return TextFieldValue(text, TextRange(a + piece.length))
}
