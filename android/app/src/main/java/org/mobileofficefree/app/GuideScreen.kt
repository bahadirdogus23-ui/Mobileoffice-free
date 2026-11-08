package org.mobileofficefree.app

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties

/** One part of the guide: a heading and its lines ("- " starts a bullet). */
internal data class GuidePart(val title: String, val lines: List<String>)

/** The guide's text ("# " starts a part) as parts. */
internal fun guideParts(text: String): List<GuidePart> {
    val parts = mutableListOf<GuidePart>()
    for (line in text.lines().map { it.trim() }.filter { it.isNotEmpty() }) {
        if (line.startsWith("# ")) parts += GuidePart(line.removePrefix("# ").trim(), emptyList())
        else if (parts.isNotEmpty()) parts[parts.lastIndex] = parts.last().let { it.copy(lines = it.lines + line) }
    }
    return parts
}

/** The user guide (res/raw/guide.txt, Turkish in raw-tr), full screen. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GuideScreen(onDismiss: () -> Unit) {
    val context = LocalContext.current
    val parts = remember {
        guideParts(context.resources.openRawResource(R.raw.guide).use { it.readBytes().toString(Charsets.UTF_8) })
    }
    Dialog(onDismissRequest = onDismiss, properties = DialogProperties(usePlatformDefaultWidth = false)) {
        Scaffold(
            topBar = {
                TopAppBar(
                    title = { Text(stringResource(R.string.guide)) },
                    navigationIcon = { IconButton(onClick = onDismiss, modifier = Modifier.described(stringResource(R.string.back))) { Text("←", fontSize = 22.sp) } },
                )
            },
        ) { padding ->
            Column(
                Modifier.padding(padding).fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 20.dp, vertical = 8.dp),
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                for (part in parts) {
                    Text(part.title, style = MaterialTheme.typography.titleMedium, color = MaterialTheme.colorScheme.primary, modifier = Modifier.padding(top = 14.dp))
                    for (line in part.lines) {
                        if (line.startsWith("- ")) {
                            Row(Modifier.fillMaxWidth()) {
                                Text("•", modifier = Modifier.widthIn(min = 16.dp))
                                Text(line.removePrefix("- "), style = MaterialTheme.typography.bodyMedium)
                            }
                        } else {
                            Text(line, style = MaterialTheme.typography.bodyMedium)
                        }
                    }
                }
            }
        }
    }
}
