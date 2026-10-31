package org.mobileofficefree.app

import android.content.ClipData
import android.content.ClipboardManager
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.OutlinedTextField
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.foundation.layout.Box
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.ui.platform.LocalContext
import java.util.Locale
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
    var fontMenu by remember { mutableStateOf(false) }
    var sizeMenu by remember { mutableStateOf(false) }
    var styleMenu by remember { mutableStateOf(false) }
    var insertMenu by remember { mutableStateOf(false) }
    var tableDialog by remember { mutableStateOf(false) }
    var replaceDialog by remember { mutableStateOf(false) }
    val pickPicture = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri -> if (uri != null) vm.insertPicture(uri) }
    if (tableDialog) {
        var rows by remember { mutableStateOf("3") }
        var cols by remember { mutableStateOf("3") }
        val r = rows.toIntOrNull()?.takeIf { it in 1..100 }
        val c = cols.toIntOrNull()?.takeIf { it in 1..20 }
        AlertDialog(
            onDismissRequest = { tableDialog = false },
            title = { Text(stringResource(R.string.insert_table)) },
            text = {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(rows, { rows = it.filter(Char::isDigit).take(3) }, label = { Text(stringResource(R.string.table_rows)) }, singleLine = true, modifier = Modifier.weight(1f), keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number))
                    OutlinedTextField(cols, { cols = it.filter(Char::isDigit).take(2) }, label = { Text(stringResource(R.string.table_cols)) }, singleLine = true, modifier = Modifier.weight(1f), keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number))
                }
            },
            confirmButton = {
                TextButton(enabled = r != null && c != null, onClick = {
                    tableDialog = false
                    if (r != null && c != null) vm.insertTable(r, c)
                }) { Text(stringResource(R.string.add)) }
            },
            dismissButton = { TextButton(onClick = { tableDialog = false }) { Text(stringResource(R.string.cancel)) } },
        )
    }
    if (replaceDialog) {
        var find by remember { mutableStateOf(vm.selectedTextHint) }
        var with by remember { mutableStateOf("") }
        AlertDialog(
            onDismissRequest = { replaceDialog = false },
            title = { Text(stringResource(R.string.replace)) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(find, { find = it }, label = { Text(stringResource(R.string.replace_find)) }, singleLine = true)
                    OutlinedTextField(with, { with = it }, label = { Text(stringResource(R.string.replace_with)) }, singleLine = true)
                }
            },
            confirmButton = {
                TextButton(enabled = find.isNotEmpty(), onClick = {
                    replaceDialog = false
                    vm.replaceAll(find, with)
                }) { Text(stringResource(R.string.replace_all)) }
            },
            dismissButton = { TextButton(onClick = { replaceDialog = false }) { Text(stringResource(R.string.cancel)) } },
        )
    }
    val context = LocalContext.current
    val clipboard = remember { context.getSystemService(ClipboardManager::class.java) }
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
                val hasSelection = vm.selection.isNotEmpty()
                TextButton(onClick = { vm.cut { clipboard.setPrimaryClip(ClipData.newPlainText("text", it)) } }, enabled = hasSelection) {
                    Text(stringResource(R.string.cut))
                }
                TextButton(onClick = { vm.copy { clipboard.setPrimaryClip(ClipData.newPlainText("text", it)) } }, enabled = hasSelection) {
                    Text(stringResource(R.string.copy))
                }
                TextButton(onClick = {
                    val item = clipboard.primaryClip?.takeIf { it.itemCount > 0 }?.getItemAt(0)
                    val text = item?.coerceToText(context)?.toString().orEmpty()
                    if (text.isNotEmpty()) vm.paste(text, item?.htmlText)
                }) { Text(stringResource(R.string.paste)) }
                Box {
                    TextButton(onClick = { fontMenu = true }) { Text(f.font?.take(12) ?: "Aa", maxLines = 1) }
                    DropdownMenu(expanded = fontMenu, onDismissRequest = { fontMenu = false }) {
                        for (name in FONTS) {
                            DropdownMenuItem(text = { Text(name) }, onClick = {
                                fontMenu = false
                                vm.setFont(name)
                            })
                        }
                    }
                }
                Box {
                    TextButton(onClick = { sizeMenu = true }) { Text(f.size?.let { "%.0f".format(Locale.ROOT, it) } ?: "–") }
                    DropdownMenu(expanded = sizeMenu, onDismissRequest = { sizeMenu = false }) {
                        for (size in SIZES) {
                            DropdownMenuItem(text = { Text(size.toString()) }, onClick = {
                                sizeMenu = false
                                vm.setSize(size.toFloat())
                            })
                        }
                    }
                }
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
                Box {
                    TextButton(onClick = { styleMenu = true }) { Text(stringResource(R.string.style)) }
                    DropdownMenu(expanded = styleMenu, onDismissRequest = { styleMenu = false }) {
                        for ((label, id) in STYLES) {
                            DropdownMenuItem(text = { Text(stringResource(label)) }, onClick = {
                                styleMenu = false
                                vm.command(id)
                            })
                        }
                    }
                }
                Box {
                    TextButton(onClick = { insertMenu = true }) { Text(stringResource(R.string.insert)) }
                    DropdownMenu(expanded = insertMenu, onDismissRequest = { insertMenu = false }) {
                        DropdownMenuItem(text = { Text(stringResource(R.string.insert_picture)) }, onClick = {
                            insertMenu = false
                            pickPicture.launch(arrayOf("image/*"))
                        })
                        DropdownMenuItem(text = { Text(stringResource(R.string.insert_table)) }, onClick = {
                            insertMenu = false
                            tableDialog = true
                        })
                        DropdownMenuItem(text = { Text(stringResource(R.string.insert_page_break)) }, onClick = {
                            insertMenu = false
                            vm.command("text.pageBreak")
                        })
                    }
                }
                TextButton(onClick = { replaceDialog = true }) { Text(stringResource(R.string.replace)) }
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

/** Paragraph styles offered: the engine's built-in Normal and Heading 1–3. */
private val STYLES = listOf(
    R.string.style_normal to "para.normal",
    R.string.style_heading1 to "para.heading1",
    R.string.style_heading2 to "para.heading2",
    R.string.style_heading3 to "para.heading3",
)

/** Fonts offered: the bundled ones, and the common Office names (drawn with a matching bundled one). */
private val FONTS = listOf("Calibri", "Arial", "Times New Roman", "Source Sans 3", "Source Serif 4", "Inter", "JetBrains Mono")
private val SIZES = listOf(8, 9, 10, 11, 12, 14, 16, 18, 20, 24, 28, 36, 48)

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
