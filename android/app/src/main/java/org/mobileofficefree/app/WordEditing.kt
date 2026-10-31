package org.mobileofficefree.app

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.FilledIconToggleButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.dp

/**
 * Stands in the keyboard's input field so that deleting it can be told apart from deleting a
 * typed letter: the field is never empty, and losing this character means "backspace".
 */
internal const val SENTINEL = "\u200B"
internal val EMPTY_INPUT = TextFieldValue(SENTINEL, TextRange(1))

/**
 * What the keyboard sends, turned into engine commands. A word being composed (predictive
 * keyboards, Turkish suggestions) stays in the field until the keyboard commits it, then is typed
 * into the document at once; deleting past the start of the field is a backspace in the document.
 */
fun keyboardEdit(new: TextFieldValue, type: (String) -> Unit, backspace: () -> Unit): TextFieldValue {
    if (!new.text.startsWith(SENTINEL)) {
        // The stand-in went: one backspace, and anything else in the field is typed.
        val rest = new.text.replace(SENTINEL, "")
        if (rest.isEmpty()) backspace() else type(rest)
        return EMPTY_INPUT
    }
    val typed = new.text.substring(SENTINEL.length)
    if (new.composition != null && typed.isNotEmpty()) return new
    if (typed.isNotEmpty()) type(typed)
    return EMPTY_INPUT
}

/** The editing tool bar and the keyboard's input, shown while a Word document is being edited. */
@Composable
fun WordEditBar(vm: WordViewModel, onSave: () -> Unit) {
    var field by remember { mutableStateOf(EMPTY_INPUT) }
    val focus = remember { FocusRequester() }
    val keyboard = LocalSoftwareKeyboardController.current
    LaunchedEffect(vm.editing) {
        if (vm.editing) {
            focus.requestFocus()
            keyboard?.show()
        }
    }
    Surface(color = MaterialTheme.colorScheme.surfaceContainer, modifier = Modifier.fillMaxWidth()) {
        Column {
            Row(
                Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                val f = vm.format
                FilledIconToggleButton(checked = f.bold, onCheckedChange = { vm.command("format.bold") }) {
                    Text("K", fontWeight = FontWeight.Bold)
                }
                FilledIconToggleButton(checked = f.italic, onCheckedChange = { vm.command("format.italic") }) {
                    Text("İ", fontStyle = FontStyle.Italic)
                }
                FilledIconToggleButton(checked = f.underline, onCheckedChange = { vm.command("format.underline") }) {
                    Text("A", textDecoration = TextDecoration.Underline)
                }
                TextButton(onClick = { vm.command("para.alignLeft") }) { Text("⇤") }
                TextButton(onClick = { vm.command("para.alignCenter") }) { Text("≡") }
                TextButton(onClick = { vm.command("para.alignRight") }) { Text("⇥") }
                TextButton(onClick = { vm.command("para.bullets") }) { Text("•") }
                TextButton(onClick = { vm.command("para.numbering") }) { Text("1.") }
                TextButton(onClick = { vm.command("edit.undo") }) { Text(stringResource(R.string.undo)) }
                TextButton(onClick = { vm.command("edit.redo") }) { Text(stringResource(R.string.redo)) }
                TextButton(onClick = onSave, enabled = vm.dirty) { Text(stringResource(R.string.save)) }
                TextButton(onClick = {
                    keyboard?.hide()
                    vm.stopEditing()
                }) { Text(stringResource(R.string.done)) }
            }
            BasicTextField(
                value = field,
                onValueChange = { new ->
                    field = keyboardEdit(new, type = vm::type, backspace = { vm.command("text.backspace") })
                },
                keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Sentences),
                textStyle = MaterialTheme.typography.bodyMedium.copy(color = MaterialTheme.colorScheme.onSurfaceVariant),
                cursorBrush = SolidColor(MaterialTheme.colorScheme.primary),
                modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 6.dp).focusRequester(focus),
            )
        }
    }
}

/** The caret and the selection drawn over page [page], which is [pageWidth] points wide. */
@Composable
fun WordCaretOverlay(vm: WordViewModel, page: Int, pageWidth: Float) {
    val caret = vm.caret?.takeIf { it.page.toInt() == page }
    val rects = vm.selection.filter { it.page.toInt() == page }
    if (caret == null && rects.isEmpty()) return
    val color = MaterialTheme.colorScheme.primary
    Canvas(Modifier.fillMaxSize()) {
        val k = size.width / pageWidth
        for (r in rects) {
            drawRect(Color(0x552196F3), topLeft = Offset(r.x * k, r.y * k), size = Size(r.width * k, r.height * k))
        }
        if (caret != null && rects.isEmpty()) {
            drawRect(color, topLeft = Offset(caret.x * k - 1f, caret.top * k), size = Size(2.dp.toPx(), caret.height * k))
        }
    }
}
