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
