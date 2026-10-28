package org.mobileofficefree.app

import java.text.Normalizer
import java.util.Locale

/**
 * A search form of this text in which "ışık", "IŞIK", "isik" and "İSİK" are all the same:
 * accents dropped, dotless and dotted i merged, lower case. Each character folds to exactly one,
 * so an index into the folded text is an index into the original.
 */
fun String.searchFold(): String {
    val out = CharArray(length)
    for (i in indices) {
        val c = this[i]
        out[i] = when (c) {
            'ı', 'I', 'İ' -> 'i'
            else -> if (c.code < 0x80) c.lowercaseChar() else Normalizer.normalize(c.toString(), Normalizer.Form.NFD)[0].toString().lowercase(Locale.ROOT).singleOrNull() ?: c
        }
    }
    return String(out)
}
