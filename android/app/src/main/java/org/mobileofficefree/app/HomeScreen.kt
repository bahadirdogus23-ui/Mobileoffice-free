package org.mobileofficefree.app

import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.runtime.rememberCoroutineScope
import kotlinx.coroutines.launch
import android.Manifest
import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.text.format.Formatter
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.core.content.edit
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.ui.draw.clip
import java.text.Collator
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
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
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
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.layout.PaddingValues
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

/** Orders the home list can be shown in; the choice is remembered. */
internal enum class HomeSort(val label: Int) {
    NEWEST(R.string.sort_newest),
    OLDEST(R.string.sort_oldest),
    NAME_AZ(R.string.sort_name_az),
    NAME_ZA(R.string.sort_name_za),
    LARGEST(R.string.sort_largest),
    SMALLEST(R.string.sort_smallest),
    KIND(R.string.sort_kind),
}

private fun List<HomeItem>.sortedBy(sort: HomeSort): List<HomeItem> {
    val collator = Collator.getInstance(Locale.forLanguageTag("tr")).apply { strength = Collator.SECONDARY }
    val byName = Comparator<HomeItem> { a, b -> collator.compare(a.name, b.name) }
    return when (sort) {
        HomeSort.NEWEST -> sortedByDescending { it.date }
        HomeSort.OLDEST -> sortedBy { it.date }
        HomeSort.NAME_AZ -> sortedWith(byName)
        HomeSort.NAME_ZA -> sortedWith(byName.reversed())
        HomeSort.LARGEST -> sortedByDescending { it.size }
        HomeSort.SMALLEST -> sortedBy { it.size }
        HomeSort.KIND -> sortedWith(compareBy<HomeItem> { it.kind.ordinal }.then(byName))
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
fun HomeScreen(onOpen: (Uri) -> Unit, onNewSheet: () -> Unit, onDiagnostics: () -> Unit, engineVersion: String, reading: ReadingSettings) {
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
    val mergePdfs = rememberPdfMerge(onOpen)
    var showSettings by remember { mutableStateOf(false) }
    if (showSettings) {
        SettingsSheet(
            reading = reading,
            engineVersion = engineVersion,
            onDiagnostics = onDiagnostics,
            onDismiss = { showSettings = false },
            onRecentsCleared = { files = recents.all() },
        )
    }
    val scan = rememberScanner(onOpen)
    // "New": a blank Word document or presentation written where the person chooses, then opened.
    var newMenu by remember { mutableStateOf(false) }
    var newKind by remember { mutableStateOf("docx") }
    val newScope = rememberCoroutineScope()
    val resources = androidx.compose.ui.platform.LocalResources.current
    val newFile = rememberLauncherForActivityResult(CreateFile()) { dest ->
        if (dest == null) return@rememberLauncherForActivityResult
        val ext = newKind
        newScope.launch {
            val ok = withContext(kotlinx.coroutines.Dispatchers.IO) {
                runCatching {
                    val bytes = org.mobileofficefree.engine.blankDocument(ext)
                    context.contentResolver.openOutputStream(dest, "wt")?.use { it.write(bytes) } ?: error("cannot write")
                }
            }
            ok.fold(
                onSuccess = { onOpen(dest) },
                onFailure = { android.widget.Toast.makeText(context, resources.getString(R.string.save_failed, it.message ?: ""), android.widget.Toast.LENGTH_LONG).show() },
            )
        }
    }
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
    val prefs = remember { context.getSharedPreferences("home", android.content.Context.MODE_PRIVATE) }
    var sort by remember {
        mutableStateOf(HomeSort.entries.firstOrNull { it.name == prefs.getString("sort", null) } ?: HomeSort.NEWEST)
    }
    var sortMenu by remember { mutableStateOf(false) }
    val shown = items.filter {
        (current.kind == null || it.kind == current.kind) &&
            (current.kind != null || extra == null || extra.matches(it)) &&
            (needle.isEmpty() || needle in it.name.searchFold())
    }.sortedBy(sort)

    Scaffold(
        floatingActionButton = {
            Box {
                ExtendedFloatingActionButton(onClick = { newMenu = true }, text = { Text(stringResource(R.string.new_document)) }, icon = { Text("＋", fontSize = 20.sp) })
                DropdownMenu(expanded = newMenu, onDismissRequest = { newMenu = false }) {
                    DropdownMenuItem(text = { Text("📝  " + stringResource(R.string.new_word)) }, onClick = {
                        newMenu = false
                        newKind = "docx"
                        newFile.launch(CreateFile.Request(resources.getString(R.string.new_word_name), "application/vnd.openxmlformats-officedocument.wordprocessingml.document"))
                    })
                    DropdownMenuItem(text = { Text("📊  " + stringResource(R.string.new_spreadsheet)) }, onClick = {
                        newMenu = false
                        onNewSheet()
                    })
                    DropdownMenuItem(text = { Text("📽  " + stringResource(R.string.new_slides)) }, onClick = {
                        newMenu = false
                        newKind = "pptx"
                        newFile.launch(CreateFile.Request(resources.getString(R.string.new_slides_name), "application/vnd.openxmlformats-officedocument.presentationml.presentation"))
                    })
                }
            }
        },
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
    ) { padding ->
        Column(Modifier.padding(padding).fillMaxSize()) {
            // Large title and settings, as iPhone screens open.
            Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.padding(start = 20.dp, top = 18.dp, end = 12.dp)) {
                Column(Modifier.weight(1f)) {
                    Text(stringResource(R.string.home_title), style = MaterialTheme.typography.headlineMedium)
                    Text(
                        stringResource(R.string.home_tagline),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 2,
                    )
                }
                Box(
                    Modifier.size(40.dp).clip(CircleShape).background(MaterialTheme.colorScheme.surfaceContainerHigh).clickable { showSettings = true },
                    contentAlignment = Alignment.Center,
                ) { Text("⚙", fontSize = 20.sp, color = MaterialTheme.colorScheme.primary) }
            }
            // Search, as a soft pill.
            SearchPill(query, onChange = { query = it }, modifier = Modifier.padding(horizontal = 16.dp, vertical = 10.dp))
            // Quick actions: lively tiles.
            Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                QuickTile("📷", stringResource(R.string.scan), Color(0xFF0A84FF), Modifier.weight(1f), scan)
                QuickTile("🖼", stringResource(R.string.images_to_pdf), Color(0xFFFF9F0A), Modifier.weight(1f), imagesToPdf)
                QuickTile("📑", stringResource(R.string.merge_short), Color(0xFFFF375F), Modifier.weight(1f), mergePdfs)
                QuickTile("📂", stringResource(R.string.open_short), Color(0xFF34C759), Modifier.weight(1f)) { pick.launch(DocumentKind.openableTypes) }
            }
            Spacer(Modifier.height(10.dp))
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
            Segmented(
                listOf(stringResource(R.string.all_documents), stringResource(R.string.recently_opened)),
                source,
                { source = it },
                Modifier.padding(horizontal = 16.dp, vertical = 10.dp),
            )
            if (shown.isNotEmpty()) {
                Row(
                    Modifier.fillMaxWidth().padding(start = 16.dp, end = 4.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        stringResource(R.string.documents_count, shown.size),
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.weight(1f),
                    )
                    Box {
                        TextButton(onClick = { sortMenu = true }) {
                            Text(stringResource(R.string.sort_by, stringResource(sort.label)) + " ▾")
                        }
                        DropdownMenu(expanded = sortMenu, onDismissRequest = { sortMenu = false }) {
                            for (s in HomeSort.entries) {
                                DropdownMenuItem(
                                    text = { Text((if (s == sort) "✓ " else "") + stringResource(s.label)) },
                                    onClick = {
                                        sortMenu = false
                                        sort = s
                                        prefs.edit { putString("sort", s.name) }
                                    },
                                )
                            }
                        }
                    }
                }
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
                LazyColumn(Modifier.fillMaxSize(), contentPadding = PaddingValues(start = 16.dp, end = 16.dp, bottom = 96.dp)) {
                    itemsIndexed(shown, key = { _, f -> f.uri.toString() }) { i, f ->
                        // One white rounded group, as iPhone lists: the first and last rows round it off.
                        val r = 16.dp
                        val shape = RoundedCornerShape(topStart = if (i == 0) r else 0.dp, topEnd = if (i == 0) r else 0.dp, bottomStart = if (i == shown.lastIndex) r else 0.dp, bottomEnd = if (i == shown.lastIndex) r else 0.dp)
                        Column(Modifier.clip(shape).background(MaterialTheme.colorScheme.surfaceContainerLow)) {
                            DocumentRow(
                                file = f,
                                onOpen = {
                                    FileInfo.rememberName(f.uri, f.name)
                                    onOpen(f.uri)
                                },
                                onShare = {
                                    val send = Intent(Intent.ACTION_SEND)
                                        .setType(context.contentResolver.getType(f.uri) ?: "*/*")
                                        .putExtra(Intent.EXTRA_STREAM, shareable(context, f.uri))
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
                            if (i < shown.lastIndex) HorizontalDivider(Modifier.padding(start = 72.dp), color = MaterialTheme.colorScheme.outlineVariant)
                        }
                    }
                    item(key = "support") { SupportCard(Modifier.padding(vertical = 16.dp)) }
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
            }
            if (file.progress > 0) {
                // How far it was read: a thin bar under the name.
                val track = MaterialTheme.colorScheme.surfaceVariant
                val fill = MaterialTheme.colorScheme.primary
                Box(Modifier.padding(top = 6.dp).width(96.dp).height(3.dp).clip(RoundedCornerShape(50)).background(track)) {
                    Box(Modifier.fillMaxHeight().fillMaxWidth(file.progress.coerceIn(0, 100) / 100f).background(fill))
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
        Spacer(Modifier.height(24.dp))
        SupportCard()
    }
}

/** A soft, pill-shaped search field. */
@Composable
private fun SearchPill(query: String, onChange: (String) -> Unit, modifier: Modifier = Modifier) {
    val colors = MaterialTheme.colorScheme
    Row(
        modifier.fillMaxWidth().clip(RoundedCornerShape(12.dp)).background(colors.surfaceVariant).padding(horizontal = 12.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text("🔍", fontSize = 15.sp, color = colors.onSurfaceVariant)
        Spacer(Modifier.width(8.dp))
        Box(Modifier.weight(1f)) {
            if (query.isEmpty()) Text(stringResource(R.string.search_files), color = colors.onSurfaceVariant, style = MaterialTheme.typography.bodyLarge)
            BasicTextField(
                value = query,
                onValueChange = onChange,
                singleLine = true,
                textStyle = MaterialTheme.typography.bodyLarge.copy(color = colors.onSurface),
                cursorBrush = SolidColor(colors.primary),
                modifier = Modifier.fillMaxWidth(),
            )
        }
        if (query.isNotEmpty()) {
            Text("✕", color = colors.onSurfaceVariant, modifier = Modifier.clip(CircleShape).clickable { onChange("") }.padding(4.dp))
        }
    }
}

/** A lively action tile: a glyph on a coloured circle over its name. */
@Composable
private fun QuickTile(glyph: String, label: String, tint: Color, modifier: Modifier = Modifier, onClick: () -> Unit) {
    Column(
        modifier.clip(RoundedCornerShape(16.dp)).background(MaterialTheme.colorScheme.surfaceContainerLow).clickable(onClick = onClick).padding(vertical = 12.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Box(Modifier.size(40.dp).clip(CircleShape).background(tint.copy(alpha = 0.16f)), contentAlignment = Alignment.Center) {
            Text(glyph, fontSize = 20.sp)
        }
        Text(label, style = MaterialTheme.typography.labelMedium, maxLines = 1, color = MaterialTheme.colorScheme.onSurface)
    }
}

/** A content address other apps can open: files listed by path go through the app's provider. */
private fun shareable(context: android.content.Context, uri: Uri): Uri =
    if (uri.scheme == "file") {
        runCatching { androidx.core.content.FileProvider.getUriForFile(context, "${context.packageName}.files", java.io.File(uri.path!!)) }.getOrDefault(uri)
    } else {
        uri
    }
