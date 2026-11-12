package org.mobileofficefree.app

import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import java.util.Locale

/** Turkish upper case (i → İ, ı → I), as function names are written. */
private fun upperTr(s: String): String = s.uppercase(Locale.forLanguageTag("tr"))

/** Whether the cursor is inside a string or a quoted sheet name (doubled quotes escape). */
private fun insideFormulaQuote(value: TextFieldValue): Boolean {
    val end = value.selection.start.coerceIn(0, value.text.length)
    var quote: Char? = null
    var i = 0
    while (i < end) {
        val c = value.text[i]
        if (quote == null) {
            if (c == '"' || c == '\'') quote = c
        } else if (c == quote) {
            if (i + 1 < end && value.text[i + 1] == quote) i++ else quote = null
        }
        i++
    }
    return quote != null
}

/** The function name being typed just before the cursor in a formula, or null. */
fun nameBeforeCursor(value: TextFieldValue): String? {
    val text = value.text
    if (!text.startsWith("=") || insideFormulaQuote(value)) return null
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

/** Whether a tap on a cell now puts its reference into the formula (Excel's "point" mode). */
fun pointing(value: TextFieldValue): Boolean = value.text.startsWith("=") && !insideFormulaQuote(value)

/**
 * [value] with the reference [ref] put at the cursor. A reference just put there by the previous
 * tap ([last], its range in the text) is replaced, so tapping around picks the cell.
 * Returns the new value and where the reference now is.
 */
fun insertReference(value: TextFieldValue, ref: String, last: TextRange?): Pair<TextFieldValue, TextRange> {
    val text = value.text
    val cursor = value.selection.start.coerceIn(0, text.length)
    val (a, b) = if (last != null && last.end == cursor && last.start in 0..cursor && last.end <= text.length) {
        last.start to last.end
    } else {
        value.selection.min.coerceIn(0, text.length) to value.selection.max.coerceIn(0, text.length)
    }
    val out = text.substring(0, a) + ref + text.substring(b)
    return TextFieldValue(out, TextRange(a + ref.length)) to TextRange(a, a + ref.length)
}

/** The function whose brackets the cursor is inside (the innermost), or null. */
fun functionAtCursor(value: TextFieldValue): String? {
    val text = value.text
    if (!text.startsWith("=")) return null
    var depth = 0
    var i = value.selection.start.coerceIn(0, text.length) - 1
    var inQuotes = text.substring(0, i + 1).count { it == '"' } % 2 == 1
    while (i >= 0) {
        val c = text[i]
        if (c == '"') inQuotes = !inQuotes
        if (!inQuotes) {
            if (c == ')') depth++
            if (c == '(') {
                if (depth == 0) {
                    var s = i
                    while (s > 0 && (text[s - 1].isLetterOrDigit() || text[s - 1] == '_' || text[s - 1] == '.')) s--
                    return text.substring(s, i).takeIf { it.isNotEmpty() && it.first().isLetter() }?.let(::upperTr)
                }
                depth--
            }
        }
        i--
    }
    return null
}
