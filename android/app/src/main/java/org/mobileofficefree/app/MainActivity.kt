package org.mobileofficefree.app

import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.view.KeyEvent
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext

class MainActivity : ComponentActivity() {
    private val sheet: SheetViewModel by viewModels()
    private val word: WordViewModel by viewModels()
    private val pdf: PdfViewModel by viewModels()
    private val slides: SlidesViewModel by viewModels()
    private val cad: CadViewModel by viewModels()

    /** A file handed over by "Open with"; AppRoot routes it by type, then clears it. */
    private var openRequest by mutableStateOf<Uri?>(null)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        if (savedInstanceState == null) handle(intent)
        setContent {
            MobileOfficeTheme { AppRoot(sheet, word, pdf, slides, cad, openRequest, onOpenHandled = { openRequest = null }) }
        }
    }

    /** Set while a viewer turns pages with the volume keys; called with +1 (next) or -1 (previous). */
    var volumeKeyHandler: ((Int) -> Unit)? = null

    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        val handler = volumeKeyHandler
        val step = when (event.keyCode) {
            KeyEvent.KEYCODE_VOLUME_DOWN -> 1
            KeyEvent.KEYCODE_VOLUME_UP -> -1
            else -> 0
        }
        if (handler == null || step == 0) return super.dispatchKeyEvent(event)
        // Consume the key-up too, so the volume does not change.
        if (event.action == KeyEvent.ACTION_DOWN) handler(step)
        return true
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handle(intent)
    }

    /** "Open with" / "Edit" from file managers, mail and messaging apps. */
    private fun handle(intent: Intent?) {
        if (intent?.action == Intent.ACTION_VIEW || intent?.action == Intent.ACTION_EDIT) {
            openRequest = intent.data
        }
    }
}

@Composable
fun MobileOfficeTheme(content: @Composable () -> Unit) {
    val dark = isSystemInDarkTheme()
    val colors = when {
        Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> {
            val context = LocalContext.current
            if (dark) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        }
        dark -> darkColorScheme()
        else -> lightColorScheme()
    }
    MaterialTheme(colorScheme = colors, content = content)
}
