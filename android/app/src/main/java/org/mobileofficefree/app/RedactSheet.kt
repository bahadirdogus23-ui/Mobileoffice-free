package org.mobileofficefree.app

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import org.mobileofficefree.engine.SensitiveKind

private val KINDS = listOf(
    SensitiveKind.TURKISH_ID to R.string.redact_turkish_id,
    SensitiveKind.IBAN to R.string.redact_iban,
    SensitiveKind.PHONE to R.string.redact_phone,
    SensitiveKind.EMAIL to R.string.redact_email,
)

/**
 * Finding personal data to black out, then blacking it out for good. [marks] is how many places
 * are marked and waiting.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun RedactSheet(
    marks: Int,
    busy: Boolean,
    onDismiss: () -> Unit,
    onMark: (List<SensitiveKind>, List<String>) -> Unit,
    onApply: () -> Unit,
    onClear: () -> Unit,
) {
    val chosen = remember { mutableStateListOf(*KINDS.map { it.first }.toTypedArray()) }
    var words by rememberSaveable { mutableStateOf("") }
    var confirm by remember { mutableStateOf(false) }
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(Modifier.verticalScroll(rememberScrollState()).padding(horizontal = 24.dp)) {
            Text(stringResource(R.string.redact_title), style = MaterialTheme.typography.titleLarge)
            Text(stringResource(R.string.redact_hint), style = MaterialTheme.typography.bodyMedium, modifier = Modifier.padding(vertical = 8.dp))
            for ((kind, label) in KINDS) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.fillMaxWidth().clickable { if (kind in chosen) chosen.remove(kind) else chosen.add(kind) },
                ) {
                    Checkbox(checked = kind in chosen, onCheckedChange = { if (it) chosen.add(kind) else chosen.remove(kind) })
                    Text(stringResource(label))
                }
            }
            OutlinedTextField(
                value = words,
                onValueChange = { words = it },
                label = { Text(stringResource(R.string.redact_words)) },
                placeholder = { Text(stringResource(R.string.redact_words_hint)) },
                minLines = 2,
                modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp),
            )
            val wordList = words.lines().map { it.trim() }.filter { it.isNotEmpty() }
            OutlinedButton(
                onClick = { onMark(chosen.toList(), wordList) },
                enabled = !busy && (chosen.isNotEmpty() || wordList.isNotEmpty()),
                modifier = Modifier.fillMaxWidth(),
            ) { Text(stringResource(R.string.redact_find)) }
            if (marks > 0) {
                Text(
                    pluralStringResource(R.plurals.redact_waiting, marks, marks),
                    style = MaterialTheme.typography.bodyMedium,
                    modifier = Modifier.padding(vertical = 8.dp),
                )
                Button(
                    onClick = { confirm = true },
                    enabled = !busy,
                    colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error),
                    modifier = Modifier.fillMaxWidth(),
                ) { Text(stringResource(R.string.redact_apply)) }
                TextButton(onClick = onClear, enabled = !busy, modifier = Modifier.fillMaxWidth()) { Text(stringResource(R.string.redact_clear)) }
            }
            Spacer(Modifier.height(24.dp))
        }
    }
    if (confirm) {
        AlertDialog(
            onDismissRequest = { confirm = false },
            title = { Text(stringResource(R.string.redact_apply)) },
            text = { Text(stringResource(R.string.redact_confirm)) },
            confirmButton = {
                TextButton(onClick = {
                    confirm = false
                    onApply()
                }) { Text(stringResource(R.string.redact_apply_short)) }
            },
            dismissButton = { TextButton(onClick = { confirm = false }) { Text(stringResource(R.string.cancel)) } },
        )
    }
}
