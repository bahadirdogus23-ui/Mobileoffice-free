package org.mobileofficefree.app

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import java.util.Locale

/**
 * The one "unsaved changes" question every editor asks before leaving (closing the file, or the
 * text editor's "Done"). It offers exactly: "Save" over the opened file ([onSave]; left out when
 * that is not allowed, with [blockedReason] saying why in one line), "Save as…" a new file,
 * "Exit without saving", and "Keep editing" (also a tap outside or Back), which stays.
 */
@Composable
internal fun UnsavedChangesDialog(
    onSave: (() -> Unit)?,
    blockedReason: String?,
    onSaveAs: () -> Unit,
    onDiscard: () -> Unit,
    onStay: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onStay,
        title = { Text(stringResource(R.string.unsaved_title)) },
        text = {
            Column {
                Text(stringResource(R.string.unsaved_body))
                if (onSave == null && blockedReason != null) {
                    Spacer(Modifier.height(8.dp))
                    Text(blockedReason, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        },
        // One column of buttons, the safest first: four side by side do not fit a phone, and a
        // wrapped row made the old "Cancel / Don't save / Save as" hard to tell apart.
        confirmButton = {
            Column(horizontalAlignment = Alignment.End, verticalArrangement = Arrangement.spacedBy(4.dp)) {
                if (onSave != null) {
                    Button(onClick = onSave) { Text(stringResource(R.string.save)) }
                    OutlinedButton(onClick = onSaveAs) { Text(stringResource(R.string.save_as)) }
                } else {
                    Button(onClick = onSaveAs) { Text(stringResource(R.string.save_as)) }
                }
                TextButton(onClick = onDiscard) {
                    Text(stringResource(R.string.leave_discard), color = MaterialTheme.colorScheme.error)
                }
                TextButton(onClick = onStay) { Text(stringResource(R.string.leave_stay)) }
            }
        },
    )
}

/**
 * "Save" on a file the engine cannot write back in its own format (an .xls, .csv, .ods, .doc,
 * .ppt…): before the system's "Save as" opens, says in plain words that a new [target] copy is
 * made and that the opened file stays as it is. Offers "Save as (.[target])" and "Cancel" (also a
 * tap outside or Back). [alsoNote] is a second reason, such as a file not read whole.
 */
@Composable
internal fun SaveAsCopyDialog(
    ext: String,
    target: String,
    alsoNote: String?,
    onSaveAs: () -> Unit,
    onCancel: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onCancel,
        title = { Text(stringResource(R.string.save_copy_title)) },
        text = {
            Column {
                Text(stringResource(R.string.save_copy_body, formatName(ext), ".$target"))
                if (alsoNote != null) {
                    Spacer(Modifier.height(8.dp))
                    Text(alsoNote, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        },
        confirmButton = { Button(onClick = onSaveAs) { Text(stringResource(R.string.save_copy_confirm, ".$target")) } },
        dismissButton = { TextButton(onClick = onCancel) { Text(stringResource(R.string.save_copy_cancel)) } },
    )
}

/** What a file with extension [ext] is called for the person: "old Excel (.xls)", "CSV (.csv)"… */
@Composable
private fun formatName(ext: String): String {
    val dotted = "." + ext.lowercase(Locale.ROOT)
    return when (ext.lowercase(Locale.ROOT)) {
        "xls", "xlt" -> stringResource(R.string.format_old_excel, dotted)
        "xlsb" -> stringResource(R.string.format_binary_excel, dotted)
        "doc", "dot" -> stringResource(R.string.format_old_word, dotted)
        "ppt", "pps", "pot" -> stringResource(R.string.format_old_powerpoint, dotted)
        "ods", "odp", "odt", "fods" -> stringResource(R.string.format_opendocument, dotted)
        "csv", "tsv", "txt" -> stringResource(R.string.format_delimited, dotted)
        else -> dotted
    }
}
