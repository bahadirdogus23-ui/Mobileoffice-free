package org.mobileofficefree.app

import android.text.format.Formatter
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp

/** The files inside a ZIP; tapping one opens it in its viewer. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ArchiveScreen(vm: ArchiveViewModel, state: ArchiveUiState, onOpen: (android.net.Uri) -> Unit) {
    val context = LocalContext.current
    Column(Modifier.fillMaxSize()) {
        TopAppBar(
            title = {
                Column {
                    Text(state.fileName, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    Text(stringResource(R.string.archive_info, state.entries.size), style = MaterialTheme.typography.bodySmall)
                }
            },
            navigationIcon = {
                IconButton(onClick = vm::close) {
                    Icon(painterResource(R.drawable.ic_arrow_back), contentDescription = stringResource(R.string.back))
                }
            },
        )
        if (vm.extracting) LinearProgressIndicator(Modifier.fillMaxWidth())
        LazyColumn(Modifier.fillMaxSize()) {
            items(state.entries, key = { it.index }) { e ->
                val kind = DocumentKind.of(e.name, null)
                Row(
                    Modifier
                        .fillMaxWidth()
                        .clickable(enabled = !vm.extracting) { vm.openEntry(e, onOpen) }
                        .padding(horizontal = 16.dp, vertical = 10.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    KindBadge(kind.badge(), kind.color())
                    Spacer(Modifier.width(16.dp))
                    Column(Modifier.weight(1f)) {
                        Text(e.name, maxLines = 1, overflow = TextOverflow.Ellipsis, style = MaterialTheme.typography.titleMedium)
                        val details = listOfNotNull(
                            e.folder.ifEmpty { null },
                            if (e.size >= 0) Formatter.formatShortFileSize(context, e.size) else null,
                        ).joinToString(" · ")
                        if (details.isNotEmpty()) {
                            Text(
                                details,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }
                Box(Modifier.padding(start = 76.dp)) { HorizontalDivider() }
            }
        }
    }
}
