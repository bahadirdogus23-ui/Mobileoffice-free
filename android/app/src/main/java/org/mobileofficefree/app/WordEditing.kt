package org.mobileofficefree.app

import androidx.compose.foundation.verticalScroll
import org.json.JSONObject
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.key
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.IntOffset
import kotlin.math.roundToInt
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
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.FilledTonalButton
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
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.sp

/**
 * Stands in the keyboard's input field so that deleting it can be told apart from deleting a
 * typed letter: the field is never empty, and losing this character means "backspace".
 */
internal const val SENTINEL = "\u200B"
internal val EMPTY_INPUT = TextFieldValue(SENTINEL, TextRange(1))

/**
 * What the keyboard sends, turned into engine edits. The field mirrors the text typed since the
 * caret last moved, so the keyboard sees real context (capitals, suggestions); every change to it
 * (a letter, a word still being composed, an autocorrection) is applied to the document at once as
 * "delete this many characters before the caret, then type this". Deleting past the start of the
 * field is a backspace in the document.
 *
 * [old] is the field as it was, [new] as the keyboard left it; returns what the field shows now.
 */
fun keyboardEdit(old: TextFieldValue, new: TextFieldValue, replace: (delete: Int, text: String) -> Unit): TextFieldValue {
    val before = old.text.removePrefix(SENTINEL)
    if (!new.text.startsWith(SENTINEL)) {
        // The stand-in went: everything typed since, and one character before it, is deleted.
        val rest = new.text.replace(SENTINEL, "")
        replace(before.codePointCount(0, before.length) + 1, rest)
        return EMPTY_INPUT
    }
    val after = new.text.substring(SENTINEL.length)
    var same = 0
    val limit = minOf(before.length, after.length)
    while (same < limit && before[same] == after[same]) same++
    if (same > 0 && same < before.length && Character.isLowSurrogate(before[same])) same--
    val delete = before.codePointCount(same, before.length)
    val typed = after.substring(same)
    if (delete > 0 || typed.isNotEmpty()) replace(delete, typed)
    // A finished line (or a long run) starts the field afresh, keeping it small.
    return if (new.composition == null && (after.endsWith('\n') || after.length > MAX_FIELD)) EMPTY_INPUT else new
}

private const val MAX_FIELD = 200

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
    var sizeDialog by remember { mutableStateOf(false) }
    var pageSheet by remember { mutableStateOf(false) }
    if (pageSheet) WordPageSheet(vm, onDismiss = { pageSheet = false })
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
    if (sizeDialog) {
        // Sizes in centimetres, as people measure paper; the engine takes points.
        var width by remember { mutableStateOf("") }
        var height by remember { mutableStateOf("") }
        fun cm(t: String) = t.replace(',', '.').toDoubleOrNull()?.takeIf { it > 0 && it < 60 }
        AlertDialog(
            onDismissRequest = { sizeDialog = false },
            title = { Text(stringResource(R.string.table_size)) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(stringResource(R.string.table_size_hint), style = MaterialTheme.typography.bodySmall)
                    OutlinedTextField(
                        value = width, onValueChange = { width = it }, singleLine = true,
                        label = { Text(stringResource(R.string.table_col_width_cm)) },
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                    )
                    OutlinedTextField(
                        value = height, onValueChange = { height = it }, singleLine = true,
                        label = { Text(stringResource(R.string.table_row_height_cm)) },
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                    )
                }
            },
            confirmButton = {
                TextButton(
                    enabled = cm(width) != null || cm(height) != null,
                    onClick = {
                        sizeDialog = false
                        cm(width)?.let { vm.command("table.columnWidth", "{\"width\": ${it * POINTS_PER_CM}}") }
                        cm(height)?.let { vm.command("table.rowHeight", "{\"height\": ${it * POINTS_PER_CM}, \"rule\": \"atLeast\"}") }
                    },
                ) { Text(stringResource(R.string.apply)) }
            },
            dismissButton = { TextButton(onClick = { sizeDialog = false }) { Text(stringResource(R.string.cancel)) } },
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
    LaunchedEffect(vm.editing, vm.keyboardAsks) {
        if (vm.editing) {
            focus.requestFocus()
            keyboard?.show()
        }
    }
    LaunchedEffect(vm.inputEpoch) { field = EMPTY_INPUT }
    var colorMenu by remember { mutableStateOf(false) }
    var highlightMenu by remember { mutableStateOf(false) }
    var tableMenu by remember { mutableStateOf(false) }
    Surface(color = MaterialTheme.colorScheme.surfaceContainer, modifier = Modifier.fillMaxWidth()) {
        // Above the system's navigation buttons (and the keyboard, which lifts the screen).
        Column(Modifier.navigationBarsPadding()) {
            // A tapped picture: sizing, in steps or to a width.
            if (vm.objectSelected) {
                Row(
                    Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 12.dp, vertical = 4.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    Text(stringResource(R.string.picture_selected), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
                    FilledTonalButton(onClick = { vm.sizePicture(scale = 80) }) { Text("− " + stringResource(R.string.picture_smaller)) }
                    FilledTonalButton(onClick = { vm.sizePicture(scale = 125) }) { Text("+ " + stringResource(R.string.picture_larger)) }
                    OutlinedButton(onClick = { vm.sizePicture(width = PAGE_TEXT_WIDTH_PT) }) { Text(stringResource(R.string.picture_page_width)) }
                    OutlinedButton(onClick = { vm.sizePicture(original = true) }) { Text(stringResource(R.string.picture_original)) }
                }
            }
            Row(
                Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 4.dp, vertical = 2.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                val f = vm.format
                val hasSelection = vm.selection.isNotEmpty()
                Tool("⌨", R.string.tool_keyboard) {
                    focus.requestFocus()
                    keyboard?.show()
                }
                Tool("↶", R.string.undo) { vm.command("edit.undo") }
                Tool("↷", R.string.redo) { vm.command("edit.redo") }
                ToolDivider()
                Box {
                    Tool(f.font?.take(10) ?: "Aa", R.string.tool_font, glyphSize = 15) { fontMenu = true }
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
                    Tool(f.size?.let { "%.0f".format(Locale.ROOT, it) } ?: "–", R.string.tool_size) { sizeMenu = true }
                    DropdownMenu(expanded = sizeMenu, onDismissRequest = { sizeMenu = false }) {
                        for (size in SIZES) {
                            DropdownMenuItem(text = { Text(size.toString()) }, onClick = {
                                sizeMenu = false
                                vm.setSize(size.toFloat())
                            })
                        }
                    }
                }
                Tool("A−", R.string.tool_smaller, glyphSize = 16) { vm.command("format.shrinkFont") }
                Tool("A+", R.string.tool_bigger, glyphSize = 16) { vm.command("format.growFont") }
                Tool("K", R.string.tool_bold, on = f.bold, weight = FontWeight.Bold) { vm.command("format.bold") }
                Tool("İ", R.string.tool_italic, on = f.italic, style = FontStyle.Italic) { vm.command("format.italic") }
                Tool("A", R.string.tool_underline, on = f.underline, decoration = TextDecoration.Underline) { vm.command("format.underline") }
                Box {
                    Tool("A", R.string.tool_text_color, glyphColor = f.color?.let(::hexColor), bar = f.color?.let(::hexColor) ?: Color.Red) { colorMenu = true }
                    DropdownMenu(expanded = colorMenu, onDismissRequest = { colorMenu = false }) {
                        DropdownMenuItem(text = { Text(stringResource(R.string.color_automatic)) }, onClick = {
                            colorMenu = false
                            vm.setColor("auto")
                        })
                        Swatches(TEXT_COLORS) { hex ->
                            colorMenu = false
                            vm.setColor(hex)
                        }
                    }
                }
                Box {
                    Tool("ab", R.string.tool_highlight, bar = Color(0xFFFFEB3B)) { highlightMenu = true }
                    DropdownMenu(expanded = highlightMenu, onDismissRequest = { highlightMenu = false }) {
                        DropdownMenuItem(text = { Text(stringResource(R.string.highlight_none)) }, onClick = {
                            highlightMenu = false
                            vm.setHighlight("none")
                        })
                        Swatches(HIGHLIGHTS.map { it.second }) { hex ->
                            highlightMenu = false
                            vm.setHighlight(HIGHLIGHTS.first { it.second == hex }.first)
                        }
                    }
                }
                ToolDivider()
                Tool("⇤", R.string.tool_left) { vm.command("para.alignLeft") }
                Tool("≡", R.string.tool_center) { vm.command("para.alignCenter") }
                Tool("⇥", R.string.tool_right) { vm.command("para.alignRight") }
                Tool("•", R.string.tool_bullets) { vm.command("para.bullets") }
                Tool("1.", R.string.tool_numbering) { vm.command("para.numbering") }
                Box {
                    Tool("¶", R.string.style) { styleMenu = true }
                    DropdownMenu(expanded = styleMenu, onDismissRequest = { styleMenu = false }) {
                        for ((label, id) in STYLES) {
                            DropdownMenuItem(text = { Text(stringResource(label)) }, onClick = {
                                styleMenu = false
                                vm.command(id)
                            })
                        }
                    }
                }
                ToolDivider()
                Tool("✂", R.string.tool_cut, enabled = hasSelection) { vm.cut { clipboard.setPrimaryClip(ClipData.newPlainText("text", it)) } }
                Tool("⧉", R.string.tool_copy, enabled = hasSelection) { vm.copy { clipboard.setPrimaryClip(ClipData.newPlainText("text", it)) } }
                Tool("📋", R.string.tool_paste, glyphSize = 16) {
                    val item = clipboard.primaryClip?.takeIf { it.itemCount > 0 }?.getItemAt(0)
                    val text = item?.coerceToText(context)?.toString().orEmpty()
                    if (text.isNotEmpty()) vm.paste(text, item?.htmlText)
                }
                Box {
                    Tool("+", R.string.insert) { insertMenu = true }
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
                Box {
                    Tool("▦", R.string.table_menu) { tableMenu = true }
                    DropdownMenu(expanded = tableMenu, onDismissRequest = { tableMenu = false }) {
                        DropdownMenuItem(text = { Text(stringResource(R.string.table_size)) }, onClick = {
                            tableMenu = false
                            sizeDialog = true
                        })
                        for ((label, id, params) in TABLE_COMMANDS) {
                            DropdownMenuItem(text = { Text(stringResource(label)) }, onClick = {
                                tableMenu = false
                                vm.command(id, params)
                            })
                        }
                    }
                }
                Tool("⇄", R.string.replace) { replaceDialog = true }
                Tool("📄", R.string.page_layout, glyphSize = 16) { pageSheet = true }
                ToolDivider()
                Tool("💾", R.string.save, enabled = vm.dirty, glyphSize = 16, onClick = onSave)
                Tool("✓", R.string.done) {
                    keyboard?.hide()
                    vm.stopEditing()
                }
            }
            // The keyboard's field: invisible, it only carries what is typed to the document.
            BasicTextField(
                value = field,
                onValueChange = { new -> field = keyboardEdit(field, new, vm::replaceTyped) },
                keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Sentences),
                textStyle = MaterialTheme.typography.bodySmall.copy(color = Color.Transparent),
                cursorBrush = SolidColor(Color.Transparent),
                modifier = Modifier.fillMaxWidth().height(1.dp).focusRequester(focus),
            )
        }
    }
}

/** A tool: a large sign over its name, so what it does is plain. [on] marks a format in force. */
@Composable
internal fun Tool(
    glyph: String,
    label: Int,
    on: Boolean = false,
    enabled: Boolean = true,
    glyphSize: Int = 20,
    weight: FontWeight? = null,
    style: FontStyle? = null,
    decoration: TextDecoration? = null,
    glyphColor: Color? = null,
    bar: Color? = null,
    onClick: () -> Unit,
) {
    val colors = MaterialTheme.colorScheme
    val content = if (enabled) colors.onSurface else colors.onSurface.copy(alpha = 0.38f)
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        modifier = Modifier
            .padding(horizontal = 1.dp)
            .clip(RoundedCornerShape(10.dp))
            .background(if (on) colors.secondaryContainer else Color.Transparent)
            .clickable(enabled = enabled, onClick = onClick)
            .widthIn(min = 52.dp)
            .padding(horizontal = 6.dp, vertical = 4.dp),
    ) {
        Text(
            glyph,
            fontSize = glyphSize.sp,
            fontWeight = weight,
            fontStyle = style,
            textDecoration = decoration,
            color = if (enabled) glyphColor ?: content else content,
            maxLines = 1,
        )
        if (bar != null) Box(Modifier.size(width = 20.dp, height = 3.dp).background(bar))
        Text(stringResource(label), fontSize = 11.sp, color = content, maxLines = 1)
    }
}

@Composable
private fun ToolDivider() {
    Box(Modifier.padding(horizontal = 4.dp).size(width = 1.dp, height = 36.dp).background(MaterialTheme.colorScheme.outlineVariant))
}

/** A grid of colour swatches (`RRGGBB`), five to a row. */
@Composable
private fun Swatches(colors: List<String>, onPick: (String) -> Unit) {
    Column(Modifier.padding(horizontal = 12.dp, vertical = 4.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        for (row in colors.chunked(5)) {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for (hex in row) {
                    Box(
                        Modifier
                            .size(32.dp)
                            .clip(CircleShape)
                            .background(hexColor(hex))
                            .border(1.dp, MaterialTheme.colorScheme.outline, CircleShape)
                            .clickable { onPick(hex) },
                    )
                }
            }
        }
    }
}

internal fun hexColor(hex: String): Color = Color(0xFF000000 or hex.toLong(16))

/** About an A4 page's text width with 2.5 cm margins, in points. */
private const val PAGE_TEXT_WIDTH_PT = 450.0

/** Table commands offered while the caret is in a table: label, engine command, parameters. */
private const val POINTS_PER_CM = 72.0 / 2.54

private val TABLE_COMMANDS = listOf(
    Triple(R.string.table_row_above, "table.insertRowAbove", ""),
    Triple(R.string.table_row_below, "table.insertRowBelow", ""),
    Triple(R.string.table_col_left, "table.insertColumnLeft", ""),
    Triple(R.string.table_col_right, "table.insertColumnRight", ""),
    Triple(R.string.table_delete_row, "table.deleteRow", ""),
    Triple(R.string.table_delete_col, "table.deleteColumn", ""),
    Triple(R.string.table_fit_window, "table.autofit", "{\"mode\": \"window\"}"),
    Triple(R.string.table_fit_contents, "table.autofit", "{\"mode\": \"contents\"}"),
    Triple(R.string.table_even_cols, "table.distributeColumns", ""),
    Triple(R.string.table_delete, "table.deleteTable", ""),
)

/** Office's standard text colours. */
private val TEXT_COLORS = listOf(
    "000000", "404040", "7F7F7F", "C00000", "FF0000",
    "FFC000", "FFFF00", "92D050", "00B050", "00B0F0",
    "0070C0", "002060", "7030A0", "1F3864", "833C0B",
)

/** The highlighter colours Word has, by the engine's name. */
private val HIGHLIGHTS = listOf(
    "yellow" to "FFFF00", "brightGreen" to "00FF00", "turquoise" to "00FFFF", "pink" to "FF00FF", "blue" to "0000FF",
    "red" to "FF0000", "darkBlue" to "000080", "teal" to "008080", "green" to "008000", "violet" to "800080",
    "darkRed" to "800000", "darkYellow" to "808000", "gray50" to "808080", "gray25" to "C0C0C0", "black" to "000000",
)

/** Paragraph styles offered: the engine's built-in Normal and Heading 1–3. */
private val STYLES = listOf(
    R.string.style_normal to "para.normal",
    R.string.style_heading1 to "para.heading1",
    R.string.style_heading2 to "para.heading2",
    R.string.style_heading3 to "para.heading3",
)

/** Fonts offered: the bundled ones, and the common Office names (drawn with a matching bundled one). */
private val FONTS = listOf("Calibri", "Arial", "Times New Roman", "Source Sans 3", "Source Serif 4", "Inter", "JetBrains Mono")
private val SIZES = listOf(8, 9, 10, 11, 12, 14, 16, 18, 20, 22, 24, 26, 28, 32, 36, 40, 48, 54, 60, 72)

/** The caret and the selection drawn over page [page], which is [pageWidth] points wide. */
@Composable
fun WordCaretOverlay(vm: WordViewModel, page: Int, pageWidth: Float, showCaret: Boolean = true) {
    val caret = vm.caret?.takeIf { showCaret && it.page.toInt() == page }
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
    SelectionHandles(vm, page, pageWidth)
}

/**
 * Android's selection handles at both ends of the selection: dragging one moves that end while
 * the other stays where it is.
 */
@Composable
private fun SelectionHandles(vm: WordViewModel, page: Int, pageWidth: Float) {
    val all = vm.selection
    if (all.isEmpty()) return
    val first = all.first()
    val last = all.last()
    val color = MaterialTheme.colorScheme.primary
    val density = LocalDensity.current
    val handle = 22.dp
    val handlePx = with(density) { handle.toPx() }
    BoxWithConstraints(Modifier.fillMaxSize()) {
        val k = constraints.maxWidth / pageWidth
        // (is the start handle, the end it moves, the end that stays)
        val ends = listOfNotNull(
            if (first.page.toInt() == page) Triple(true, first, last) else null,
            if (last.page.toInt() == page) Triple(false, last, first) else null,
        )
        for ((start, moving, fixed) in ends) {
            key(start) {
                // While a handle is dragged it follows the finger from where it was picked up,
                // and the other end stays as it was then.
                var grab by remember { mutableStateOf<HandleGrab?>(null) }
                val movingNow by rememberUpdatedState(moving)
                val fixedNow by rememberUpdatedState(fixed)
                val atX = if (start) moving.x else moving.x + moving.width
                val atY = moving.y + moving.height
                val g = grab
                val left = if (g != null) g.x * k + g.drag.x - (if (start) handlePx else 0f) else atX * k - (if (start) handlePx else 0f)
                val top = if (g != null) g.y * k + g.drag.y else atY * k
                Box(
                    Modifier
                        .offset { IntOffset(left.roundToInt(), top.roundToInt()) }
                        .size(handle)
                        .background(
                            color,
                            if (start) RoundedCornerShape(50, 0, 50, 50) else RoundedCornerShape(0, 50, 50, 50),
                        )
                        .pointerInput(start) {
                            detectDragGestures(
                                onDragStart = {
                                    val m = movingNow
                                    val f = fixedNow
                                    grab = HandleGrab(
                                        x = if (start) m.x else m.x + m.width, y = m.y + m.height, lineHeight = m.height,
                                        fixedPage = f.page.toInt(),
                                        fx = if (start) f.x + f.width else f.x,
                                        fy = f.y + f.height / 2,
                                    )
                                },
                                onDragEnd = { grab = null },
                                onDragCancel = { grab = null },
                            ) { change, amount ->
                                change.consume()
                                val now = grab ?: return@detectDragGestures
                                val moved = now.copy(drag = now.drag + amount)
                                grab = moved
                                // The finger is under the text: aim at the line just above the handle.
                                val px = moved.x + moved.drag.x / k
                                val py = moved.y - moved.lineHeight / 2 + moved.drag.y / k
                                vm.selectBetween(moved.fixedPage, moved.fx, moved.fy, page, px, py)
                            }
                        },
                )
            }
        }
    }
}

/** A selection handle being dragged: where it was picked up (points), how far it went (pixels) and the end that stays. */
private data class HandleGrab(
    val x: Float,
    val y: Float,
    val lineHeight: Float,
    val fixedPage: Int,
    val fx: Float,
    val fy: Float,
    val drag: Offset = Offset.Zero,
)

/** Word's page setup: orientation, paper, margins, columns, header, footer and page numbers. */
@OptIn(androidx.compose.material3.ExperimentalMaterial3Api::class)
@Composable
private fun WordPageSheet(vm: WordViewModel, onDismiss: () -> Unit) {
    var header by remember { mutableStateOf("") }
    var footer by remember { mutableStateOf("") }
    androidx.compose.material3.ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(Modifier.padding(horizontal = 20.dp).verticalScroll(rememberScrollState())) {
            Text(stringResource(R.string.page_layout), style = MaterialTheme.typography.titleLarge)
            PageChoice(stringResource(R.string.pdf_orientation)) {
                PageChip(stringResource(R.string.pdf_portrait)) { vm.command("layout.orientation", """{"value":"portrait"}""") }
                PageChip(stringResource(R.string.pdf_landscape)) { vm.command("layout.orientation", """{"value":"landscape"}""") }
            }
            PageChoice(stringResource(R.string.pdf_paper)) {
                for (name in listOf("A4", "A5", "A3", "Letter", "Legal")) PageChip(name) { vm.command("layout.size", JSONObject().put("name", name).toString()) }
            }
            PageChoice(stringResource(R.string.pdf_margins)) {
                for ((preset, label) in listOf("normal" to R.string.pdf_margins_normal, "narrow" to R.string.pdf_margins_narrow, "moderate" to R.string.margins_moderate, "wide" to R.string.pdf_margins_wide)) {
                    PageChip(stringResource(label)) { vm.command("layout.margins", JSONObject().put("preset", preset).toString()) }
                }
            }
            PageChoice(stringResource(R.string.page_columns)) {
                for (n in 1..3) PageChip("$n") { vm.command("layout.columns", JSONObject().put("count", n).put("apply", "document").toString()) }
            }
            PageChoice(stringResource(R.string.page_numbers)) {
                for ((align, label) in listOf("left" to R.string.align_left, "center" to R.string.align_center, "right" to R.string.align_right)) {
                    PageChip(stringResource(label)) { vm.command("insert.pageNumber", JSONObject().put("position", "bottom").put("align", align).toString()) }
                }
            }
            Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.padding(top = 8.dp)) {
                OutlinedTextField(header, { header = it }, singleLine = true, label = { Text(stringResource(R.string.page_header)) }, modifier = Modifier.weight(1f))
                TextButton(onClick = { vm.command("insert.header", JSONObject().put("text", header).toString()) }, enabled = header.isNotBlank()) { Text(stringResource(R.string.apply)) }
            }
            Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.padding(top = 4.dp, bottom = 24.dp)) {
                OutlinedTextField(footer, { footer = it }, singleLine = true, label = { Text(stringResource(R.string.page_footer)) }, modifier = Modifier.weight(1f))
                TextButton(onClick = { vm.command("insert.footer", JSONObject().put("text", footer).toString()) }, enabled = footer.isNotBlank()) { Text(stringResource(R.string.apply)) }
            }
        }
    }
}

@Composable
private fun PageChoice(title: String, chips: @Composable () -> Unit) {
    Text(title, style = MaterialTheme.typography.labelLarge, modifier = Modifier.padding(top = 12.dp, bottom = 4.dp))
    Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) { chips() }
}

@Composable
private fun PageChip(label: String, onClick: () -> Unit) {
    androidx.compose.material3.AssistChip(onClick = onClick, label = { Text(label) })
}
