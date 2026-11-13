package org.mobileofficefree.app

import java.io.File
import java.io.IOException

internal data class MeasurementRecord(val kind: String, val name: String, val values: List<Long>)

/** A damaged record must not prevent later records from being loaded. Accept legacy durations. */
internal fun parseMeasurementRecord(line: String): MeasurementRecord? {
    val parts = line.split('\t')
    if (parts.size < 2 || parts[1].isBlank()) return null
    if (!((parts[0] == "S" && parts.size in 7..8) || (parts[0] == "J" && parts.size == 6))) return null
    val values = parts.drop(2).map { it.toLongOrNull()?.takeIf { n -> n >= 0 } ?: return null }
    return MeasurementRecord(parts[0], parts[1], values)
}

/** Replace only a complete snapshot; report failures so the caller can retry. */
internal fun writeMeasurementSnapshot(file: File, text: String) {
    val temporary = File(file.parentFile, "${file.name}.tmp")
    temporary.writeText(text)
    if (!temporary.renameTo(file)) throw IOException("Cannot replace measurement snapshot")
}
