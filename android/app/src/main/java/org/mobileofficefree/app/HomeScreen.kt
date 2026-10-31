package org.mobileofficefree.app

import android.Manifest
import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.text.format.Formatter
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.LifecycleResumeEffect
import java.text.DateFormat
import java.util.Date
import java.util.Locale
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** Bottom tabs: all recent files, then one per document family. */
private enum class HomeTab(val label: Int, val badge: String, val kind: DocumentKind?) {
    ALL(R.string.tab_home, "⌂", null),
    WORD(R.string.tab_word, "W", DocumentKind.WORD),
    SHEET(R.string.tab_excel, "X", DocumentKind.SHEET),
    SLIDES(R.string.tab_powerpoint, "P", DocumentKind.SLIDES),
    PDF(R.string.tab_pdf, "PDF", DocumentKind.PDF),
}

/** One colour per document family, used for badges and tabs. */
fun DocumentKind.color(): Color = when (this) {
    DocumentKind.WORD -> Color(0xFF2B6CD4)
    DocumentKind.SHEET -> Color(0xFF1E8E4E)
    DocumentKind.SLIDES -> Color(0xFFD0581F)
    DocumentKind.PDF -> Color(0xFFD03A35)
    DocumentKind.CAD -> Color(0xFF0E8FA8)
    DocumentKind.IMAGE -> Color(0xFF8E44AD)
    DocumentKind.TEXT -> Color(0xFF546E7A)
    DocumentKind.ARCHIVE -> Color(0xFF8D6E63)
    DocumentKind.INVOICE -> Color(0xFF3949AB)
    DocumentKind.OTHER -> Color(0xFF6B7280)
}

internal fun DocumentKind.badge(): String = when (this) {
    DocumentKind.WORD -> "W"
    DocumentKind.SHEET -> "X"
    DocumentKind.SLIDES -> "P"
    DocumentKind.PDF -> "PDF"
    DocumentKind.CAD -> "DWG"
    DocumentKind.IMAGE -> "IMG"
    DocumentKind.TEXT -> "TXT"
    DocumentKind.ARCHIVE -> "ZIP"
    DocumentKind.INVOICE -> "FTR"
    DocumentKind.OTHER -> "?"
}

private val UDF_COLOR = Color(0xFFB71C1C)

/** Quick filters on the home tab for the families the bottom tabs do not have. */
private enum class ExtraFilter(val label: Int) {
    UDF(R.string.filter_udf),
    INVOICE(R.string.filter_invoice),
    CAD(R.string.filter_cad),
    ARCHIVE(R.string.filter_archive),
    TEXT(R.string.filter_text);

    fun matches(item: HomeItem): Boolean = when (this) {
        UDF -> item.name.endsWith(".udf", ignoreCase = true)
        INVOICE -> item.kind == DocumentKind.INVOICE
        CAD -> item.kind == DocumentKind.CAD
        ARCHIVE -> item.kind == DocumentKind.ARCHIVE
        TEXT -> item.kind == DocumentKind.TEXT
    }
}

/** One row of the home list, from a device scan or the recent-files list. */
private data class HomeItem(
    val uri: Uri,
    val name: String,
    val kind: DocumentKind,
    val size: Long,
    val date: Long,
    val progress: Int,
)

@Composable
fun HomeScreen(onOpen: (Uri) -> Unit, onNewSheet: () -> Unit, onDiagnostics: () -> Unit, engineVersion: String) {
    val context = LocalContext.current
    val recents = remember { RecentFiles(context) }
    // Re-read whenever the home screen comes back (it leaves composition while a document is open).
    var files by remember { mutableStateOf(recents.all()) }
    var access by remember { mutableStateOf(DeviceFiles.hasAccess(context)) }
    var device by remember { mutableStateOf<List<DeviceFile>?>(null) }
    var refresh by remember { mutableIntStateOf(0) }
    var tab by rememberSaveable { mutableIntStateOf(0) }
    var source by rememberSaveable { mutableIntStateOf(0) } // 0: all documents, 1: recently opened
    var query by rememberSaveable { mutableStateOf("") }
    // A narrower family on the home tab: UDF, e-invoices, drawings, archives, text.
    var extraName by rememberSaveable { mutableStateOf<String?>(null) }
    val extra = extraName?.let { n -> ExtraFilter.entries.firstOrNull { it.name == n } }
    val folderAccess = remember { FolderAccess(context) }
    var folders by remember { mutableStateOf(folderAccess.all()) }
    val pickFolder = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocumentTree()) { tree ->
        if (tree != null) {
            folderAccess.add(tree)
            folders = folderAccess.all()
        }
    }
    val pick = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        if (uri != null) onOpen(uri)
    }
    val imagesToPdf = rememberImagesToPdf(onOpen)
    var showSettings by remember { mutableStateOf(false) }
    if (showSettings) SettingsSheet(onDismiss = { showSettings = false }, onRecentsCleared = { files = recents.all() })
    val scan = rememberScanner(onOpen)
    val askLegacy = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        access = granted
    }
    val askAccess: () -> Unit = {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            try {
                context.startActivity(DeviceFiles.accessSettingsIntent(context))
            } catch (_: ActivityNotFoundException) {
                context.startActivity(DeviceFiles.fallbackSettingsIntent())
            }
        } else {
            askLegacy.launch(Manifest.permission.READ_EXTERNAL_STORAGE)
        }
    }
    // Coming back from Settings or from a document: re-check access and refresh both lists.
    LifecycleResumeEffect(Unit) {
        access = DeviceFiles.hasAccess(context)
        files = recents.all()
        refresh++
        onPauseOrDispose { }
    }
    // With "All files access" the whole device is listed; without it, the folders the person picked.
    val listsDevice = access || folders.isNotEmpty()
    LaunchedEffect(access, folders, refresh) {
        device = when {
            access -> withContext(Dispatchers.IO) { runCatching { DeviceFiles.scan(context) }.getOrNull() }
            folders.isNotEmpty() -> withContext(Dispatchers.IO) { runCatching { folderAccess.scan() }.getOrNull() }
            else -> null
        }
    }

    val current = HomeTab.entries[tab]
    val progressOf = files.associate { it.uri to it.progress }
    val items = if (source == 0 && listsDevice) {
        device.orEmpty().map { HomeItem(it.uri, it.name, it.kind, it.size, it.modifiedAt, progressOf[it.uri] ?: 0) }
    } else {
        files.map { HomeItem(it.uri, it.name, it.kind, it.size, it.openedAt, it.progress) }
    }
    val needle = query.trim().searchFold()
    val shown = items.filter {
        (current.kind == null || it.kind == current.kind) &&
            (current.kind != null || extra == null || extra.matches(it)) &&
            (needle.isEmpty() || needle in it.name.searchFold())
    }

    Scaffold(
        bottomBar = {
            NavigationBar {
                HomeTab.entries.forEachIndexed { i, t ->
                    NavigationBarItem(
                        selected = i == tab,
                        onClick = { tab = i },
                        icon = { KindBadge(t.badge, t.kind?.color() ?: MaterialTheme.colorScheme.primary, small = true) },
                        label = { Text(stringResource(t.label), maxLines = 1) },
                    )
                }
            }
        },
        floatingActionButton = {
            ExtendedFloatingActionButton(onClick = { pick.launch(DocumentKind.openableTypes) }) {
                Text(stringResource(R.string.open_file))
            }
        },
    ) { padding ->
        Column(Modifier.padding(padding).fillMaxSize()) {
            Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.padding(start = 20.dp, top = 16.dp, end = 8.dp)) {
                Text(
                    stringResource(R.string.app_name),
                    style = MaterialTheme.typography.headlineSmall,
                    fontWeight = FontWeight.SemiBold,
                    modifier = Modifier.weight(1f),
                )
                TextButton(onClick = { showSettings = true }) { Text(stringResource(R.string.settings)) }
                TextButton(onClick = onDiagnostics) { Text(stringResource(R.string.diagnostics)) }
            }
            Text(
                stringResource(R.string.home_tagline),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(horizontal = 20.dp, vertical = 4.dp),
            )
            // Making a new PDF: from the camera, or from pictures already on the phone.
            Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(onClick = scan, modifier = Modifier.weight(1f)) { Text(stringResource(R.string.scan), maxLines = 1) }
                OutlinedButton(onClick = imagesToPdf, modifier = Modifier.weight(1f)) { Text(stringResource(R.string.images_to_pdf), maxLines = 1) }
            }
            OutlinedTextField(
                value = query,
                onValueChange = { query = it },
                placeholder = { Text(stringResource(R.string.search_files)) },
                singleLine = true,
                trailingIcon = if (query.isNotEmpty()) {
                    { IconButton(onClick = { query = "" }) { Text("✕") } }
                } else {
                    null
                },
                modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
            )
            if (current.kind == null) {
                Row(
                    Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 16.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    for (f in ExtraFilter.entries) {
                        FilterChip(
                            selected = extra == f,
                            onClick = { extraName = if (extra == f) null else f.name },
                            label = { Text(stringResource(f.label)) },
                        )
                    }
                }
            }
            PrimaryTabRow(selectedTabIndex = source) {
                Tab(selected = source == 0, onClick = { source = 0 }, text = { Text(stringResource(R.string.all_documents)) })
                Tab(selected = source == 1, onClick = { source = 1 }, text = { Text(stringResource(R.string.recently_opened)) })
            }
            if (source == 0 && !access) {
                Card(Modifier.fillMaxWidth().padding(16.dp)) {
                    Column(Modifier.padding(16.dp)) {
                        Text(stringResource(R.string.all_files_title), style = MaterialTheme.typography.titleMedium)
                        Spacer(Modifier.height(4.dp))
                        Text(stringResource(R.string.all_files_body), style = MaterialTheme.typography.bodyMedium)
                        Spacer(Modifier.height(12.dp))
                        Row {
                            Button(onClick = askAccess) { Text(stringResource(R.string.all_files_button)) }
                            Spacer(Modifier.width(8.dp))
                            OutlinedButton(onClick = { pickFolder.launch(null) }) { Text(stringResource(R.string.pick_folder)) }
                        }
                        for (tree in folders) {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Text(
                                    stringResource(R.string.folder_label, folderAccess.label(tree)),
                                    style = MaterialTheme.typography.bodyMedium,
                                    modifier = Modifier.weight(1f),
                                )
                                IconButton(onClick = {
                                    folderAccess.remove(tree)
                                    folders = folderAccess.all()
                                }) { Text("✕") }
                            }
                        }
                    }
                }
            }
            if (current.kind == DocumentKind.SHEET) {
                OutlinedButton(onClick = onNewSheet, Modifier.padding(horizontal = 20.dp, vertical = 8.dp)) {
                    Text(stringResource(R.string.new_spreadsheet))
                }
            }
            if (shown.isEmpty() && needle.isNotEmpty()) {
                Text(
                    stringResource(R.string.no_search_results, query.trim()),
                    style = MaterialTheme.typography.bodyMedium,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.fillMaxWidth().padding(32.dp),
                )
            } else if (shown.isEmpty() && !(source == 0 && listsDevice && device == null)) {
                EmptyState(engineVersion)
            } else {
                LazyColumn(Modifier.fillMaxSize()) {
                    items(shown, key = { it.uri.toString() }) { f ->
                        DocumentRow(
                            file = f,
                            onOpen = { onOpen(f.uri) },
                            onShare = {
                                val send = Intent(Intent.ACTION_SEND)
                                    .setType(context.contentResolver.getType(f.uri) ?: "*/*")
                                    .putExtra(Intent.EXTRA_STREAM, f.uri)
                                    .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                                context.startActivity(Intent.createChooser(send, f.name))
                            },
                            onForget = if (source == 1 || !listsDevice) {
                                {
                                    recents.forget(f.uri)
                                    files = recents.all()
                                }
                            } else {
                                null
                            },
                        )
                        HorizontalDivider(Modifier.padding(start = 76.dp))
                    }
                }
            }
        }
    }
}

@Composable
private fun DocumentRow(file: HomeItem, onOpen: () -> Unit, onShare: () -> Unit, onForget: (() -> Unit)?) {
    val context = LocalContext.current
    var menu by remember { mutableStateOf(false) }
    val ext = file.name.substringAfterLast('.', "").uppercase(Locale.ROOT)
    val size = if (file.size >= 0) Formatter.formatShortFileSize(context, file.size) else null
    val date = DateFormat.getDateInstance(DateFormat.SHORT).format(Date(file.date))
    Row(
        Modifier.fillMaxWidth().clickable(onClick = onOpen).padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        // UYAP documents read as Word files but are a family of their own to people.
        if (ext == "UDF") KindBadge("UDF", UDF_COLOR) else KindBadge(file.kind.badge(), file.kind.color())
        Spacer(Modifier.width(16.dp))
        Column(Modifier.weight(1f)) {
            Text(file.name.substringBeforeLast('.'), maxLines = 1, overflow = TextOverflow.Ellipsis, style = MaterialTheme.typography.titleMedium)
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    listOfNotNull(ext, size, date).joinToString(" · "),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                if (file.progress > 0) {
                    Spacer(Modifier.width(8.dp))
                    Surface(shape = RoundedCornerShape(50), color = MaterialTheme.colorScheme.secondaryContainer) {
                        Text(
                            stringResource(R.string.viewed_percent, file.progress),
                            style = MaterialTheme.typography.labelSmall,
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp),
                        )
                    }
                }
            }
        }
        Box {
            IconButton(onClick = { menu = true }) { Text("⋮", fontSize = 22.sp) }
            DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                DropdownMenuItem(text = { Text(stringResource(R.string.share)) }, onClick = { menu = false; onShare() })
                if (onForget != null) {
                    DropdownMenuItem(text = { Text(stringResource(R.string.remove_from_list)) }, onClick = { menu = false; onForget() })
                }
            }
        }
    }
}

@Composable
internal fun KindBadge(text: String, color: Color, small: Boolean = false) {
    val side = if (small) 26.dp else 44.dp
    Box(
        Modifier.size(side).background(color, RoundedCornerShape(if (small) 6.dp else 10.dp)),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text,
            color = Color.White,
            fontWeight = FontWeight.Bold,
            fontSize = if (small) (if (text.length > 1) 8.sp else 13.sp) else (if (text.length > 1) 12.sp else 20.sp),
        )
    }
}

@Composable
private fun EmptyState(engineVersion: String) {
    Column(
        Modifier.fillMaxSize().padding(32.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text(stringResource(R.string.no_recent_files), style = MaterialTheme.typography.titleMedium, textAlign = TextAlign.Center)
        Spacer(Modifier.height(8.dp))
        Text(
            stringResource(R.string.privacy_note),
            style = MaterialTheme.typography.bodySmall,
            textAlign = TextAlign.Center,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(4.dp))
        Text(
            stringResource(R.string.engine_version, engineVersion),
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}
