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
    CAD(
        setOf("dwg", "dxf"),
        setOf("image/vnd.dwg", "application/acad", "application/x-dwg", "image/vnd.dxf", "application/dxf"),
    ),
    IMAGE(
        setOf("jpg", "jpeg", "png", "webp", "gif", "bmp", "heic", "heif", "avif"),
        setOf("image/jpeg", "image/png", "image/webp", "image/gif", "image/bmp", "image/x-ms-bmp", "image/heic", "image/heif", "image/avif"),
    ),
    TEXT(
        setOf("txt", "log", "json", "xml", "ini", "conf", "cfg", "yaml", "yml", "properties", "sql", "srt", "vtt", "nfo"),
        setOf("text/plain", "application/json", "text/xml", "application/xml", "application/x-subrip", "text/vtt"),
    ),
    ARCHIVE(setOf("zip"), setOf("application/zip", "application/x-zip-compressed")),
    OTHER(emptySet(), emptySet());

    companion object {
        fun of(name: String, mime: String?): DocumentKind {
            val ext = name.substringAfterLast('.', "").lowercase(Locale.ROOT)
            val m = mime?.lowercase(Locale.ROOT)
            return entries.firstOrNull { ext in it.extensions }
                ?: entries.firstOrNull { m != null && m in it.mimeTypes }
                ?: if (m != null && m.startsWith("text/")) TEXT else OTHER
        }

        /** Everything the picker offers today. */
        val openableTypes: Array<String> =
            (entries.flatMap { it.mimeTypes } + "text/*" + "application/octet-stream").toTypedArray()

        /**
         * Extensions the home screen lists from the whole device. Photos are left to the gallery and
         * configuration files to their apps; both still open from the picker and "Open with".
         */
        val listedExtensions: Set<String> =
            WORD.extensions + SHEET.extensions + SLIDES.extensions + PDF.extensions + CAD.extensions + ARCHIVE.extensions + "txt"
    }
}
