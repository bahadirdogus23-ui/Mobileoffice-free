package org.mobileofficefree.app

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import org.mobileofficefree.engine.FormFieldKind
import org.mobileofficefree.engine.PdfFormField

/** What the person entered in one field. */
sealed interface FieldInput {
    data class Text(val text: String) : FieldInput
    data class Check(val on: Boolean) : FieldInput
    data class Choice(val value: String) : FieldInput
}

/** Changes the person made: entries whose input differs from the field's current value. */
fun formChanges(fields: List<PdfFormField>, inputs: Map<String, FieldInput>): Map<String, FieldInput> =
    inputs.filter { (name, input) ->
        val f = fields.firstOrNull { it.name == name } ?: return@filter false
        when (input) {
            is FieldInput.Text -> input.text != f.value
            is FieldInput.Check -> input.on != f.value.isNotEmpty()
            is FieldInput.Choice -> input.value != f.value
        }
    }

/** The form's fields one under another, page by page, to fill in; [onApply] gets the changes. */
@Composable
fun FormSheet(fields: List<PdfFormField>, onDismiss: () -> Unit, onApply: (Map<String, FieldInput>) -> Unit) {
    val inputs = remember(fields) { mutableStateMapOf<String, FieldInput>() }
    val fillable = fields.filter { it.kind != FormFieldKind.OTHER }
    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)) {
        Row(Modifier.fillMaxWidth().padding(horizontal = 24.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(stringResource(R.string.form_fill), style = MaterialTheme.typography.titleLarge, modifier = Modifier.weight(1f))
            TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel)) }
            Button(
                onClick = { onApply(formChanges(fields, inputs)) },
                enabled = formChanges(fields, inputs).isNotEmpty(),
            ) { Text(stringResource(R.string.apply)) }
        }
        if (fillable.isEmpty()) {
            Text(stringResource(R.string.form_none), modifier = Modifier.padding(24.dp))
            return@ModalBottomSheet
        }
        LazyColumn(contentPadding = PaddingValues(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            var lastPage: UInt? = UInt.MAX_VALUE
            for (f in fillable) {
                if (f.page != lastPage) {
                    lastPage = f.page
                    f.page?.let { p ->
                        item(key = "page-$p") {
                            Text(stringResource(R.string.form_page, p.toInt() + 1), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
                        }
                    }
                }
                item(key = f.name) { FieldEditor(f, inputs) }
            }
        }
    }
}

@Composable
private fun FieldEditor(f: PdfFormField, inputs: MutableMap<String, FieldInput>) {
    when (f.kind) {
        FormFieldKind.TEXT -> OutlinedTextField(
            value = (inputs[f.name] as? FieldInput.Text)?.text ?: f.value,
            onValueChange = { inputs[f.name] = FieldInput.Text(it) },
            label = { Text(f.label) },
            enabled = !f.readOnly,
            singleLine = !f.multiline,
            minLines = if (f.multiline) 3 else 1,
            modifier = Modifier.fillMaxWidth(),
        )
        FormFieldKind.CHECK_BOX -> {
            val on = (inputs[f.name] as? FieldInput.Check)?.on ?: f.value.isNotEmpty()
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier.fillMaxWidth().clickable(enabled = !f.readOnly) { inputs[f.name] = FieldInput.Check(!on) },
            ) {
                Checkbox(checked = on, onCheckedChange = { inputs[f.name] = FieldInput.Check(it) }, enabled = !f.readOnly)
                Text(f.label)
            }
        }
        FormFieldKind.RADIO, FormFieldKind.CHOICE -> Column {
            Text(f.label, style = MaterialTheme.typography.bodyMedium)
            val chosen = (inputs[f.name] as? FieldInput.Choice)?.value ?: f.value
            Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for (o in f.options) {
                    FilterChip(
                        selected = o.value == chosen,
                        onClick = { inputs[f.name] = FieldInput.Choice(if (o.value == chosen) "" else o.value) },
                        label = { Text(o.text.ifBlank { o.value }) },
                        enabled = !f.readOnly,
                    )
                }
            }
        }
        FormFieldKind.OTHER -> {}
    }
}
