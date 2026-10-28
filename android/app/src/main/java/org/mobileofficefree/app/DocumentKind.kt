package org.mobileofficefree.app

import java.util.Locale

/** What a file is, by extension first and MIME type second (Locale.ROOT: Turkish "I" must not leak in). */
enum class DocumentKind(val extensions: Set<String>, val mimeTypes: Set<String>) {
    WORD(
        setOf("docx", "docm", "dotx", "dotm", "doc", "dot", "odt", "rtf", "md", "html", "htm"),
        setOf(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "application/vnd.ms-word.document.macroenabled.12",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.template",
            "application/msword",
            "application/vnd.oasis.opendocument.text",
            "application/rtf",
            "text/rtf",
            "text/markdown",
        ),
    ),
    SHEET(
        setOf("xlsx", "xlsm", "xltx", "xltm", "xls", "xlsb", "ods", "csv", "tsv"),
        setOf(
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "application/vnd.ms-excel.sheet.macroenabled.12",
            "application/vnd.ms-excel",
            "application/vnd.ms-excel.sheet.binary.macroenabled.12",
            "application/vnd.oasis.opendocument.spreadsheet",
            "text/csv",
            "text/comma-separated-values",
            "text/tab-separated-values",
        ),
    ),
    SLIDES(
        setOf("pptx", "ppsx", "potx", "ppt", "pps", "odp"),
        setOf(
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            "application/vnd.openxmlformats-officedocument.presentationml.slideshow",
            "application/vnd.ms-powerpoint",
            "application/vnd.oasis.opendocument.presentation",
        ),
    ),
    PDF(setOf("pdf"), setOf("application/pdf")),
    OTHER(emptySet(), emptySet());

    companion object {
        fun of(name: String, mime: String?): DocumentKind {
            val ext = name.substringAfterLast('.', "").lowercase(Locale.ROOT)
            return entries.firstOrNull { ext in it.extensions }
                ?: entries.firstOrNull { mime != null && mime.lowercase(Locale.ROOT) in it.mimeTypes }
                ?: if (name.endsWith(".txt", ignoreCase = true) || mime == "text/plain") WORD else OTHER
        }

        /** Everything the picker offers today. */
        val openableTypes: Array<String> =
            (WORD.mimeTypes + SHEET.mimeTypes + PDF.mimeTypes + SLIDES.mimeTypes + "text/plain").toTypedArray()
    }
}
