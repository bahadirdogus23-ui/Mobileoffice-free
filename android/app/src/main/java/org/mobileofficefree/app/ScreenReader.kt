package org.mobileofficefree.app

import android.content.Context
import android.view.accessibility.AccessibilityManager
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext

/**
 * Whether a screen reader explores the screen by touch (TalkBack and the like), following it
 * being turned on or off while shown. The extra work done only for screen readers
 * (page texts, one node per spreadsheet cell) waits for this, so nobody else pays for it.
 */
@Composable
fun rememberScreenReaderOn(): Boolean {
    val context = LocalContext.current
    val manager = remember(context) { context.getSystemService(Context.ACCESSIBILITY_SERVICE) as? AccessibilityManager }
    var on by remember(manager) { mutableStateOf(manager != null && manager.isEnabled && manager.isTouchExplorationEnabled) }
    DisposableEffect(manager) {
        val listener = AccessibilityManager.TouchExplorationStateChangeListener { enabled ->
            on = enabled && manager?.isEnabled == true
        }
        manager?.addTouchExplorationStateChangeListener(listener)
        on = manager != null && manager.isEnabled && manager.isTouchExplorationEnabled
        onDispose { manager?.removeTouchExplorationStateChangeListener(listener) }
    }
    return on
}
