package org.mobileofficefree.app

import android.app.Activity
import android.content.ClipData
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.IntentSender
import android.net.Uri
import android.text.format.Formatter
import android.util.Log
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.IntentSenderRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp
import java.text.DateFormat
import java.util.Date
import java.util.Locale
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** A file on the home list, as the file actions need it. */
data class FileTarget(val uri: Uri, val name: String, val kind: DocumentKind, val size: Long, val date: Long)

/** What a file action changed, for the home list to follow. */
sealed interface FileChange {
    data class Renamed(val old: Uri, val new: Uri, val name: String) : FileChange
    data class Deleted(val uris: List<Uri>) : FileChange
    data object Added : FileChange
}

/** The delete confirmation: each file and how it would go (to the trash, for good, or not from here). */
class DeletePlan(val items: List<Pair<FileTarget, DeleteWay>>)

/**
 * Rename, copy, delete, details, "Open with" and sharing for the home list's files: the dialogs,
 * the system's questions (write, trash and delete confirmations, the "Save" picker for a copy that
 * cannot go next to its file), and the bookkeeping afterwards (recent files, recovery copies). The
 * file work itself is in [FileOps], always off the main thread.
 */
@Stable
class FileActions internal constructor(
    private val context: Context,
    private val scope: CoroutineScope,
    private val changed: (FileChange) -> Unit,
) {
    var renaming by mutableStateOf<FileTarget?>(null)
        private set
    var renameError by mutableStateOf<String?>(null)
        private set
    var deleting by mutableStateOf<DeletePlan?>(null)
        private set
    /** Files the trash would not take: deleting them for good needs a second "yes". */
    var permanentAfterTrash by mutableStateOf<List<FileTarget>?>(null)
        private set
    var showing by mutableStateOf<Pair<FileTarget, FileDetails?>?>(null)
        private set
    /** A file action is running (the list shows a thin progress bar; dialogs disable their buttons). */
    var busy by mutableStateOf(false)
        private set

    internal var consentLauncher: ActivityResultLauncher<IntentSenderRequest>? = null
    internal var placeLauncher: ActivityResultLauncher<CreateFile.Request>? = null
    private var consentWait: CompletableDeferred<Boolean>? = null
    private var placeWait: CompletableDeferred<Uri?>? = null

    private fun toast(text: String) = Toast.makeText(context, text, Toast.LENGTH_LONG).show()

    internal fun consentAnswered(ok: Boolean) {
        consentWait?.complete(ok)
        consentWait = null
    }

    internal fun placeAnswered(uri: Uri?) {
        placeWait?.complete(uri)
        placeWait = null
    }

    /** Shows the system's confirmation and waits for the answer. */
    private suspend fun askConsent(sender: IntentSender): Boolean {
        val launcher = consentLauncher ?: return false
        consentWait?.complete(false)
        val wait = CompletableDeferred<Boolean>()
        consentWait = wait
        try {
            launcher.launch(IntentSenderRequest.Builder(sender).build())
        } catch (_: Exception) {
            consentWait = null
            return false
        }
        return wait.await()
    }

    /** Runs [op] off the main thread; when Android asks the person first, asks, then runs it again if needed. */
    private suspend fun perform(op: (consented: Boolean) -> FileOutcome): FileOutcome {
        val first = withContext(Dispatchers.IO) { op(false) }
        if (first !is FileOutcome.NeedsConsent) return first
        if (!askConsent(first.sender)) return FileOutcome.Cancelled
        first.granted?.let { return it }
        val again = withContext(Dispatchers.IO) { op(true) }
        return if (again is FileOutcome.NeedsConsent) FileOutcome.Failed(context.getString(R.string.file_no_permission)) else again
    }

    // --- Rename ------------------------------------------------------------------------------

    fun rename(target: FileTarget) {
        renameError = null
        renaming = target
    }

    fun dismissRename() {
        if (!busy) renaming = null
    }

    fun clearRenameError() {
        renameError = null
    }

    internal fun confirmRename(target: FileTarget, newName: String) {
        if (busy) return
        scope.launch {
            busy = true
            val outcome = try {
                perform { consented -> FileOps.rename(context, target.uri, target.name, newName, consented) }
            } finally {
                busy = false
            }
            when (outcome) {
                is FileOutcome.Done -> {
                    renaming = null
                    withContext(Dispatchers.IO) {
                        // Nothing may keep pointing at the old name or address.
                        RecentFiles(context).replace(target.uri, outcome.uri, outcome.name)
                        runCatching { RecoveryStore(context).move(target.uri, outcome.uri, outcome.name) }
                    }
                    FileInfo.rememberName(outcome.uri, outcome.name)
                    toast(context.getString(R.string.rename_done, outcome.name))
                    changed(FileChange.Renamed(target.uri, outcome.uri, outcome.name))
                }
                is FileOutcome.Failed -> renameError = outcome.message
                else -> Unit
            }
        }
    }

    // --- Copy --------------------------------------------------------------------------------

    fun duplicate(target: FileTarget) {
        if (busy) return
        scope.launch {
            busy = true
            var outcome = try {
                withContext(Dispatchers.IO) { FileOps.duplicate(context, target.uri, target.name, context.getString(R.string.copy_word)) }
            } finally {
                busy = false
            }
            if (outcome is FileOutcome.NeedsLocation) {
                toast(context.getString(R.string.duplicate_pick_place))
                val name = outcome.name
                val dest = askPlace(CreateFile.Request(name, outcome.mime, near = target.uri)) ?: return@launch
                outcome = if (dest == target.uri) {
                    FileOutcome.Failed(context.getString(R.string.duplicate_same_file))
                } else {
                    busy = true
                    try {
                        withContext(Dispatchers.IO) { FileOps.copyTo(context, target.uri, dest, name) }
                    } finally {
                        busy = false
                    }
                }
            }
            when (outcome) {
                is FileOutcome.Done -> {
                    toast(context.getString(R.string.duplicate_done, outcome.name))
                    changed(FileChange.Added)
                }
                is FileOutcome.Failed -> toast(context.getString(R.string.duplicate_failed, outcome.message))
                else -> Unit
            }
        }
    }

    private suspend fun askPlace(request: CreateFile.Request): Uri? {
        val launcher = placeLauncher ?: return null
        placeWait?.complete(null)
        val wait = CompletableDeferred<Uri?>()
        placeWait = wait
        try {
            launcher.launch(request)
        } catch (_: Exception) {
            placeWait = null
            return null
        }
        return wait.await()
    }

    // --- Delete ------------------------------------------------------------------------------

    /** Asks how each file would be deleted, then shows the confirmation. Nothing is deleted before the person confirms. */
    fun delete(targets: List<FileTarget>) {
        if (busy || targets.isEmpty()) return
        scope.launch {
            busy = true
            val plan = try {
                withContext(Dispatchers.IO) { targets.map { it to FileOps.deleteWay(context, it.uri) } }
            } finally {
                busy = false
            }
            deleting = DeletePlan(plan)
        }
    }

    fun dismissDelete() {
        deleting = null
        permanentAfterTrash = null
    }

    internal fun confirmDelete(plan: DeletePlan) {
        deleting = null
        if (busy) return
        scope.launch {
            busy = true
            try {
                val trash = plan.items.filter { it.second == DeleteWay.TRASH }.map { it.first }
                val permanent = plan.items.filter { it.second == DeleteWay.PERMANENT }.map { it.first }
                Log.i(FILES_LOG_TAG, "delete confirmed: ${trash.size} to the trash, ${permanent.size} for good")
                val trashed = ArrayList<FileTarget>()
                var trashRefused = emptyList<FileTarget>()
                if (trash.isNotEmpty()) {
                    // With "All files access" the app moves them to the trash itself (no system dialog);
                    // Android is asked only for what that did not take.
                    val direct = withContext(Dispatchers.IO) { trash.filter { FileOps.trashDirectly(context, it.uri) } }
                    trashed += direct
                    val rest = trash.filter { it !in direct }
                    if (rest.isNotEmpty()) {
                        val outcome = perform { FileOps.trash(context, rest.map { it.uri }) }
                        Log.i(FILES_LOG_TAG, "trash request for ${rest.size} file(s): ${outcome.javaClass.simpleName}")
                        when (outcome) {
                            is FileOutcome.Done -> trashed += rest
                            is FileOutcome.Failed -> trashRefused = rest
                            else -> {
                                // The person said no to the system: nothing else is deleted either.
                                finish(trashed, emptyList())
                                return@launch
                            }
                        }
                    }
                }
                val deleted = deleteForGood(permanent)
                finish(trashed, deleted)
                if (trashRefused.isNotEmpty()) {
                    // Nothing is deleted for good without the app's own second "yes".
                    Log.i(FILES_LOG_TAG, "trash refused for ${trashRefused.size} file(s): asking about deleting them for good")
                    permanentAfterTrash = trashRefused
                }
            } finally {
                busy = false
            }
        }
    }

    /** The second "yes" for files the trash would not take. */
    internal fun confirmPermanent(targets: List<FileTarget>) {
        permanentAfterTrash = null
        if (busy) return
        scope.launch {
            busy = true
            try {
                finish(emptyList(), deleteForGood(targets))
            } finally {
                busy = false
            }
        }
    }

    private suspend fun deleteForGood(targets: List<FileTarget>): List<FileTarget> {
        val deleted = ArrayList<FileTarget>()
        for (t in targets) {
            when (val o = perform { consented -> FileOps.deletePermanently(context, t.uri, consented) }) {
                is FileOutcome.Done -> deleted += t
                is FileOutcome.Failed -> toast(context.getString(R.string.delete_failed, t.name, o.message))
                is FileOutcome.Cancelled -> break
                else -> Unit
            }
        }
        return deleted
    }

    private suspend fun finish(trashed: List<FileTarget>, deleted: List<FileTarget>) {
        val gone = trashed + deleted
        if (gone.isEmpty()) return
        withContext(Dispatchers.IO) {
            val recents = RecentFiles(context)
            val recovery = RecoveryStore(context)
            for (t in gone) {
                recents.forget(t.uri)
                runCatching { recovery.clear(t.uri) }
            }
        }
        val res = context.resources
        toast(
            when {
                gone.size == 1 && trashed.size == 1 -> res.getString(R.string.delete_done_trash, gone[0].name)
                gone.size == 1 -> res.getString(R.string.delete_done, gone[0].name)
                deleted.isEmpty() -> res.getQuantityString(R.plurals.delete_many_done_trash, gone.size, gone.size)
                else -> res.getQuantityString(R.plurals.delete_many_done, gone.size, gone.size)
            },
        )
        changed(FileChange.Deleted(gone.map { it.uri }))
    }

    // --- Details, Open with, Share -------------------------------------------------------------

    fun details(target: FileTarget) {
        showing = target to null
        scope.launch {
            val d = withContext(Dispatchers.IO) { runCatching { FileOps.details(context, target.uri, target.name, target.size, target.date) }.getOrNull() }
            if (showing?.first == target) showing = target to (d ?: FileDetails(target.name, null, target.size, null, target.date))
        }
    }

    fun dismissDetails() {
        showing = null
    }

    /** "Open with another app": the system's chooser for viewing the file, this app left out, with read access granted. */
    fun openWith(target: FileTarget) {
        val uri = Share.shareable(context, target.uri)
        if (uri == null) {
            Toast.makeText(context, R.string.share_failed, Toast.LENGTH_SHORT).show()
            return
        }
        val mime = Share.mimeFromName(target.name) ?: runCatching { context.contentResolver.getType(uri) }.getOrNull() ?: "*/*"
        val view = Intent(Intent.ACTION_VIEW).setDataAndType(uri, mime).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        view.clipData = ClipData.newRawUri(target.name, uri)
        val chooser = Intent.createChooser(view, context.getString(R.string.open_with_title, target.name))
            .putExtra(Intent.EXTRA_EXCLUDE_COMPONENTS, arrayOf(ComponentName(context, MainActivity::class.java)))
        Share.start(context, chooser)
    }

    /** Shares one file, or several in one go. */
    fun share(targets: List<FileTarget>) {
        if (targets.size == 1) {
            Share.file(context, targets[0].uri, targets[0].name)
            return
        }
        val uris = targets.mapNotNull { Share.shareable(context, it.uri) }
        if (uris.isEmpty()) {
            Toast.makeText(context, R.string.share_failed, Toast.LENGTH_SHORT).show()
            return
        }
        val mimes = targets.map { Share.mimeFromName(it.name) ?: "*/*" }.distinct()
        val type = mimes.singleOrNull() ?: "*/*"
        val send = Intent(Intent.ACTION_SEND_MULTIPLE).setType(type)
            .putParcelableArrayListExtra(Intent.EXTRA_STREAM, ArrayList(uris))
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        send.clipData = ClipData(targets[0].name, arrayOf(type), ClipData.Item(uris[0])).apply {
            uris.drop(1).forEach { addItem(ClipData.Item(it)) }
        }
        Share.start(context, Intent.createChooser(send, null))
    }
}

/** The home list's file actions, with the launchers they need; [onChanged] follows each change. */
@Composable
fun rememberFileActions(onChanged: (FileChange) -> Unit): FileActions {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val latest = rememberUpdatedState(onChanged)
    val actions = remember { FileActions(context, scope) { latest.value(it) } }
    actions.consentLauncher = rememberLauncherForActivityResult(ActivityResultContracts.StartIntentSenderForResult()) {
        Log.i(FILES_LOG_TAG, "system confirmation answered: result code ${it.resultCode}")
        actions.consentAnswered(it.resultCode == Activity.RESULT_OK)
    }
    actions.placeLauncher = rememberLauncherForActivityResult(CreateFile()) { actions.placeAnswered(it) }
    return actions
}

/** The dialogs of [actions] (rename, delete, details); shown while one is asked for. */
@Composable
fun FileActionDialogs(actions: FileActions) {
    actions.renaming?.let { RenameDialog(it, actions) }
    actions.deleting?.let { DeleteDialog(it, actions) }
    actions.permanentAfterTrash?.let { targets ->
        AlertDialog(
            onDismissRequest = actions::dismissDelete,
            title = { Text(stringResource(R.string.delete_trash_failed_title)) },
            text = {
                Column(Modifier.verticalScroll(rememberScrollState())) {
                    Text(stringResource(R.string.delete_trash_failed_body, targets.joinToString("\n") { "• " + it.name }))
                }
            },
            confirmButton = {
                TextButton(onClick = { actions.confirmPermanent(targets) }) {
                    Text(stringResource(R.string.delete_permanent_confirm), color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = { TextButton(onClick = actions::dismissDelete) { Text(stringResource(R.string.cancel)) } },
        )
    }
    actions.showing?.let { (target, details) -> DetailsDialog(target, details, actions::dismissDetails) }
}

@Composable
private fun RenameDialog(target: FileTarget, actions: FileActions) {
    // The name without its extension is selected, so typing replaces just that.
    val base = FileNames.base(target.name)
    var value by remember(target) { mutableStateOf(TextFieldValue(target.name, TextRange(0, base.length))) }
    val problem = FileNames.problem(value.text)
    val newName = value.text.trim()
    val extChanged = problem == null && FileNames.extensionChanged(target.name, newName)
    val canConfirm = problem == null && newName != target.name && !actions.busy
    val focus = remember { FocusRequester() }
    LaunchedEffect(target) {
        // The dialog's window composes a moment later: ask for the keyboard once its field exists.
        kotlinx.coroutines.delay(150)
        runCatching { focus.requestFocus() }
    }
    val problemText = when (problem) {
        FileNames.Problem.EMPTY -> stringResource(R.string.rename_error_empty)
        FileNames.Problem.ILLEGAL -> stringResource(R.string.rename_error_illegal, FileNames.ILLEGAL_CHARS.toList().joinToString(" "))
        FileNames.Problem.HIDDEN -> stringResource(R.string.rename_error_hidden)
        FileNames.Problem.TOO_LONG -> stringResource(R.string.rename_error_long)
        null -> null
    }
    val message = problemText ?: actions.renameError
    AlertDialog(
        onDismissRequest = actions::dismissRename,
        title = { Text(stringResource(R.string.rename_title)) },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState())) {
                OutlinedTextField(
                    value = value,
                    onValueChange = {
                        value = it
                        actions.clearRenameError()
                    },
                    label = { Text(stringResource(R.string.rename_label)) },
                    singleLine = true,
                    isError = message != null,
                    enabled = !actions.busy,
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                    keyboardActions = KeyboardActions(onDone = { if (canConfirm) actions.confirmRename(target, newName) }),
                    modifier = Modifier.fillMaxWidth().focusRequester(focus),
                )
                if (message != null) {
                    Spacer(Modifier.height(6.dp))
                    Text(
                        message,
                        color = MaterialTheme.colorScheme.error,
                        style = MaterialTheme.typography.bodySmall,
                        modifier = Modifier.semantics { liveRegion = LiveRegionMode.Polite },
                    )
                }
                if (extChanged) {
                    Spacer(Modifier.height(6.dp))
                    val old = FileNames.extension(target.name).ifEmpty { "—" }
                    val new = FileNames.extension(newName).ifEmpty { "—" }
                    Text(
                        stringResource(R.string.rename_ext_warning, old, new),
                        color = MaterialTheme.colorScheme.tertiary,
                        style = MaterialTheme.typography.bodySmall,
                        modifier = Modifier.semantics { liveRegion = LiveRegionMode.Polite },
                    )
                }
            }
        },
        confirmButton = {
            TextButton(enabled = canConfirm, onClick = { actions.confirmRename(target, newName) }) {
                Text(stringResource(R.string.file_rename_confirm))
            }
        },
        dismissButton = { TextButton(enabled = !actions.busy, onClick = actions::dismissRename) { Text(stringResource(R.string.cancel)) } },
    )
}

@Composable
private fun DeleteDialog(plan: DeletePlan, actions: FileActions) {
    val items = plan.items
    val trash = items.count { it.second == DeleteWay.TRASH }
    val permanent = items.count { it.second == DeleteWay.PERMANENT }
    val none = items.filter { it.second == DeleteWay.NONE }.map { it.first }
    val single = items.singleOrNull()
    AlertDialog(
        onDismissRequest = actions::dismissDelete,
        title = { Text(stringResource(if (trash + permanent == 0) R.string.delete_cannot_title else R.string.delete_title)) },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState())) {
                if (single != null) {
                    val name = single.first.name
                    Text(name, fontWeight = FontWeight.Bold)
                    Spacer(Modifier.height(8.dp))
                    Text(
                        when (single.second) {
                            DeleteWay.TRASH -> stringResource(R.string.delete_trash_body, name)
                            DeleteWay.PERMANENT -> stringResource(R.string.delete_permanent_body, name)
                            DeleteWay.NONE -> stringResource(R.string.file_not_here)
                        },
                    )
                } else {
                    if (trash > 0) Text(pluralStringResource(R.plurals.delete_many_trash, trash, trash))
                    if (permanent > 0) Text(pluralStringResource(R.plurals.delete_many_permanent, permanent, permanent), color = MaterialTheme.colorScheme.error)
                    if (none.isNotEmpty()) {
                        Text(pluralStringResource(R.plurals.delete_many_skipped, none.size, none.size))
                        Text(none.joinToString("\n") { "• " + it.name }, style = MaterialTheme.typography.bodySmall)
                    }
                    Spacer(Modifier.height(8.dp))
                    Text(items.filter { it.second != DeleteWay.NONE }.joinToString("\n") { "• " + it.first.name }, style = MaterialTheme.typography.bodySmall)
                }
            }
        },
        confirmButton = {
            if (trash + permanent > 0) {
                TextButton(onClick = { actions.confirmDelete(plan) }) {
                    Text(
                        stringResource(if (permanent > 0) R.string.delete_permanent_confirm else R.string.delete_trash_confirm),
                        color = MaterialTheme.colorScheme.error,
                    )
                }
            } else {
                TextButton(onClick = actions::dismissDelete) { Text(stringResource(R.string.ok)) }
            }
        },
        dismissButton = {
            if (trash + permanent > 0) TextButton(onClick = actions::dismissDelete) { Text(stringResource(R.string.cancel)) }
        },
    )
}

@Composable
private fun DetailsDialog(target: FileTarget, details: FileDetails?, onDismiss: () -> Unit) {
    val context = LocalContext.current
    val unknown = stringResource(R.string.info_unknown)
    val d = details
    val name = d?.name ?: target.name
    val ext = FileNames.extension(name).uppercase(Locale.ROOT)
    val type = listOfNotNull(ext.ifEmpty { null }, d?.mime).joinToString(" · ").ifEmpty { unknown }
    val size = (d?.size ?: target.size).takeIf { it >= 0 }?.let { Formatter.formatFileSize(context, it) } ?: unknown
    val modified = (d?.modified ?: target.date).takeIf { it > 0 }?.let {
        DateFormat.getDateTimeInstance(DateFormat.MEDIUM, DateFormat.SHORT).format(Date(it))
    } ?: unknown
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.info_title)) },
        text = {
            SelectionContainer {
                Column(Modifier.verticalScroll(rememberScrollState())) {
                    InfoLine(stringResource(R.string.info_name), name)
                    InfoLine(stringResource(R.string.info_type), type)
                    InfoLine(stringResource(R.string.info_size), size)
                    InfoLine(stringResource(R.string.info_folder), if (d == null) "…" else d.folder ?: unknown)
                    InfoLine(stringResource(R.string.info_modified), modified)
                }
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.close)) } },
    )
}

@Composable
private fun InfoLine(label: String, value: String) {
    Column(Modifier.fillMaxWidth().semantics(mergeDescendants = true) {}) {
        Text(label, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(value, style = MaterialTheme.typography.bodyMedium)
        Spacer(Modifier.height(8.dp))
    }
}
