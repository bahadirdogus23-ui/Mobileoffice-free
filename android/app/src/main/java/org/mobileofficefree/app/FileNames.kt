package org.mobileofficefree.app

import java.util.Locale

/**
 * File-name rules for renaming and copying from the home list. Plain Kotlin (no Android types),
 * so the unit tests cover it.
 */
internal object FileNames {
    /** Why a typed name cannot be used. */
    enum class Problem { EMPTY, ILLEGAL, HIDDEN, TOO_LONG }

    /** Characters Android's shared storage (FAT/exFAT cards, FUSE) or other systems refuse in a name. */
    const val ILLEGAL_CHARS = "/\\:*?\"<>|"

    /** Longest name most file systems take, in UTF-8 bytes. */
    private const val MAX_BYTES = 255

    /** What is wrong with [name] (as typed; surrounding spaces are ignored), or null when it can be used. */
    fun problem(name: String): Problem? {
        val n = name.trim()
        return when {
            n.isEmpty() -> Problem.EMPTY
            n.any { it in ILLEGAL_CHARS || it < ' ' || it == '\u007F' } -> Problem.ILLEGAL
            // "." and ".." are not names; a leading dot hides the file from most apps (and from this list).
            n.startsWith(".") -> Problem.HIDDEN
            n.toByteArray(Charsets.UTF_8).size > MAX_BYTES -> Problem.TOO_LONG
            else -> null
        }
    }

    /** The extension without the dot ("pdf"), or "" when there is none; a leading dot does not start one. */
    fun extension(name: String): String {
        val dot = name.lastIndexOf('.')
        return if (dot <= 0 || dot == name.length - 1) "" else name.substring(dot + 1)
    }

    /** [name] without its extension. */
    fun base(name: String): String {
        val ext = extension(name)
        return if (ext.isEmpty()) name else name.substring(0, name.length - ext.length - 1)
    }

    /** True when [old] and [new] end in different extensions (letter case aside, compared with [Locale.ROOT]). */
    fun extensionChanged(old: String, new: String): Boolean =
        extension(old).lowercase(Locale.ROOT) != extension(new.trim()).lowercase(Locale.ROOT)

    /** True when [new] is [old] with only its letter case changed (or not changed at all). */
    fun sameIgnoringCase(old: String, new: String): Boolean = old.lowercase(Locale.ROOT) == new.trim().lowercase(Locale.ROOT)

    /** "Rapor (kopya).docx" for [n] = 1, "Rapor (kopya 2).docx" after that; [word] is "kopya" or "copy". */
    fun copyName(name: String, word: String, n: Int = 1): String {
        val ext = extension(name)
        val label = if (n <= 1) "($word)" else "($word $n)"
        val stem = "${base(name)} $label"
        return if (ext.isEmpty()) stem else "$stem.$ext"
    }

    /** The first [copyName] that [taken] does not report, trying up to 999 numbers. */
    fun uniqueCopyName(name: String, word: String, taken: (String) -> Boolean): String {
        for (n in 1..999) {
            val candidate = copyName(name, word, n)
            if (!taken(candidate)) return candidate
        }
        return copyName(name, word, System.currentTimeMillis().rem(100_000).toInt() + 1000)
    }
}
