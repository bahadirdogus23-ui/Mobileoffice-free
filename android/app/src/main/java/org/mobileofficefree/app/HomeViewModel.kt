package org.mobileofficefree.app

import android.content.Context
import android.os.Build
import android.provider.MediaStore
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel

/**
 * What the home screen shows, kept while a document is open. The home screen leaves composition
 * then, so its own `remember`ed state (even `rememberSaveable`) was lost: closing a TXT opened
 * under the "Text" filter came back to the first tab at the top, with an empty list that a full
 * device scan then filled again.
 */
class HomeViewModel : ViewModel() {
    /** The bottom tab (HomeTab's ordinal). */
    var tab by mutableIntStateOf(0)
    /** 0: all documents, 1: recently opened. */
    var source by mutableIntStateOf(0)
    var query by mutableStateOf("")
    /** A narrower family on the home tab (ExtraFilter's name): UDF, e-invoices, drawings, archives, text. */
    var extraName by mutableStateOf<String?>(null)

    /** The recently opened files; null until they are first read. */
    var files by mutableStateOf<List<RecentFile>?>(null)
    /** The documents found on the device; null until the first scan is done (or without access). */
    var device by mutableStateOf<List<DeviceFile>?>(null)
    /** Grows to ask for a new device scan (back from Settings or from another app). */
    var refresh by mutableIntStateOf(0)

    /**
     * Grows after a rename, copy or delete from the home list: the list is looked at again quietly
     * (MediaStore's change counter has moved), without walking the folders in full.
     */
    var changes by mutableIntStateOf(0)

    /** What [device] was last scanned for (access, folders, [refresh]): the same home list again is not scanned again in full. */
    var scannedFor: Any? = null
    /** MediaStore's change counter when [device] was scanned (Android 11+): unchanged means no file was added, changed or removed. */
    var scannedGeneration: Long? = null

    private var listKey: Any? = null
    private var listState: LazyListState? = null

    /**
     * The list's scroll position for [key] (tab, source, filter, sort): kept while a document is
     * open, started at the top when the key changes.
     */
    fun listStateFor(key: Any): LazyListState {
        val kept = listState
        if (kept != null && key == listKey) return kept
        return LazyListState().also {
            listState = it
            listKey = key
        }
    }

    companion object {
        /** MediaStore's change counter over every shared volume; null before Android 11 or when it cannot be read. Call off the main thread. */
        fun storageGeneration(context: Context): Long? =
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                runCatching { MediaStore.getExternalVolumeNames(context).sumOf { MediaStore.getGeneration(context, it) } }.getOrNull()
            } else {
                null
            }
    }
}
