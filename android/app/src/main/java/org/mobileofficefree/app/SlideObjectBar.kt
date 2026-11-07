package org.mobileofficefree.app

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import org.mobileofficefree.engine.SlideObjectKind
import org.mobileofficefree.engine.SlideStyle

/** What can be done to a picked slide object: its text's look, its box, its place among the others. */
@Composable
fun SlideObjectBar(vm: SlidesViewModel) {
    val o = vm.picked?.second ?: return
    val text = o.kind == SlideObjectKind.TEXT
    var colorMenu by remember { mutableStateOf(false) }
    var fillMenu by remember { mutableStateOf(false) }
    var alignMenu by remember { mutableStateOf(false) }
    var arrangeMenu by remember { mutableStateOf(false) }
    var fontMenu by remember { mutableStateOf(false) }
    Row(Modifier.horizontalScroll(rememberScrollState())) {
        TextButton(onClick = vm::undo, enabled = vm.canUndo) { Text("↶") }
        if (vm.pickedHasText) TextButton(onClick = vm::editPickedText) { Text(stringResource(R.string.slide_edit_text)) }
        if (text) {
            Box {
                TextButton(onClick = { fontMenu = true }) { Text("Aa " + stringResource(R.string.tool_font)) }
                DropdownMenu(expanded = fontMenu, onDismissRequest = { fontMenu = false }) {
                    for (name in DOCUMENT_FONTS) {
                        DropdownMenuItem(text = { Text(name) }, onClick = {
                            fontMenu = false
                            vm.stylePicked(SlideStyle.Font(name))
                        })
                    }
                }
            }
            TextButton(onClick = { vm.stylePicked(SlideStyle.TextSize(-2.0)) }) { Text("A−") }
            TextButton(onClick = { vm.stylePicked(SlideStyle.TextSize(2.0)) }) { Text("A+") }
            TextButton(onClick = { vm.stylePicked(SlideStyle.Bold(!vm.pickedBold)) }) {
                Text("K", fontWeight = if (vm.pickedBold) FontWeight.Black else FontWeight.Normal)
            }
            Box {
                TextButton(onClick = { colorMenu = true }) { Text(stringResource(R.string.slide_text_color)) }
                DropdownMenu(expanded = colorMenu, onDismissRequest = { colorMenu = false }) {
                    ColorRow(SLIDE_COLORS) {
                        colorMenu = false
                        vm.stylePicked(SlideStyle.TextColor(it.toUInt()))
                    }
                }
            }
            Box {
                TextButton(onClick = { alignMenu = true }) { Text(stringResource(R.string.slide_align)) }
                DropdownMenu(expanded = alignMenu, onDismissRequest = { alignMenu = false }) {
                    listOf(R.string.align_left, R.string.align_center, R.string.align_right, R.string.align_justify).forEachIndexed { i, label ->
                        DropdownMenuItem(text = { Text(stringResource(label)) }, onClick = {
                            alignMenu = false
                            vm.stylePicked(SlideStyle.Align(i.toUByte()))
                        })
                    }
                }
            }
        }
        if (o.kind != SlideObjectKind.PICTURE) {
            Box {
                TextButton(onClick = { fillMenu = true }) { Text(stringResource(R.string.slide_fill)) }
                DropdownMenu(expanded = fillMenu, onDismissRequest = { fillMenu = false }) {
                    DropdownMenuItem(text = { Text(stringResource(R.string.slide_fill_none)) }, onClick = {
                        fillMenu = false
                        vm.stylePicked(SlideStyle.Fill(null))
                    })
                    ColorRow(SLIDE_COLORS) {
                        fillMenu = false
                        vm.stylePicked(SlideStyle.Fill(it.toUInt()))
                    }
                }
            }
        }
        TextButton(onClick = { vm.placePicked(0.0, 0.0, 1.0, 90.0) }) { Text("⟳ 90°") }
        if (o.rotation != 0f) TextButton(onClick = { vm.placePicked(0.0, 0.0, 1.0, -o.rotation.toDouble()) }) { Text(stringResource(R.string.slide_upright)) }
        TextButton(onClick = vm::duplicatePicked) { Text(stringResource(R.string.slide_duplicate_object)) }
        Box {
            TextButton(onClick = { arrangeMenu = true }) { Text(stringResource(R.string.slide_arrange)) }
            DropdownMenu(expanded = arrangeMenu, onDismissRequest = { arrangeMenu = false }) {
                DropdownMenuItem(text = { Text(stringResource(R.string.slide_to_front)) }, onClick = {
                    arrangeMenu = false
                    vm.arrangePicked(true)
                })
                DropdownMenuItem(text = { Text(stringResource(R.string.slide_to_back)) }, onClick = {
                    arrangeMenu = false
                    vm.arrangePicked(false)
                })
                // To another slide, at the same place.
                val here = vm.picked?.first ?: 0
                for (n in 0 until (vm.state?.pages?.size ?: 0)) {
                    if (n == here) continue
                    DropdownMenuItem(text = { Text(stringResource(R.string.slide_move_to, n + 1)) }, onClick = {
                        arrangeMenu = false
                        vm.movePickedTo(n)
                    })
                }
            }
        }
        TextButton(onClick = vm::deletePicked) { Text(stringResource(R.string.delete)) }
        TextButton(onClick = vm::dropPick) { Text(stringResource(R.string.done)) }
    }
}

/** Colours to pick from, as round swatches in rows of six. */
@Composable
private fun ColorRow(colors: List<Int>, onPick: (Int) -> Unit) {
    for (row in colors.chunked(6)) {
        Row(Modifier.padding(horizontal = 12.dp, vertical = 4.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (c in row) {
                Box(
                    Modifier
                        .size(30.dp)
                        .background(Color(0xFF000000.toInt() or c), CircleShape)
                        .border(1.dp, MaterialTheme.colorScheme.outlineVariant, CircleShape)
                        .clickable { onPick(c) },
                )
            }
        }
    }
}

/** Office-like colours for slide text and boxes (0xRRGGBB). */
private val SLIDE_COLORS = listOf(
    0x000000, 0x404040, 0x7F7F7F, 0xBFBFBF, 0xFFFFFF, 0xC00000,
    0xFF0000, 0xFFC000, 0xFFFF00, 0x92D050, 0x00B050, 0x00B0F0,
    0x0070C0, 0x002060, 0x7030A0, 0xFF66CC, 0x833C0B, 0x1F3864,
)
