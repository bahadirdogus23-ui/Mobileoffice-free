package org.mobileofficefree.app

import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.view.KeyEvent
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.viewModels
import androidx.compose.foundation.background
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp

class MainActivity : ComponentActivity() {
    private val sheet: SheetViewModel by viewModels()
    private val word: WordViewModel by viewModels()
    private val pdf: PdfViewModel by viewModels()
    private val slides: SlidesViewModel by viewModels()
    private val cad: CadViewModel by viewModels()
    private val image: ImageViewModel by viewModels()
    private val text: TextViewModel by viewModels()
    private val archive: ArchiveViewModel by viewModels()
    private val invoice: InvoiceViewModel by viewModels()

    /** One lock for the process: turning the phone or recreating the screen does not lock again. */
    private val lock get() = processLock

    /** The system's confirm-credential screen reports here. */
    private val confirmCredential = registerForActivityResult(ActivityResultContracts.StartActivityForResult()) {
        if (it.resultCode == RESULT_OK) lock.unlocked()
    }

    /** A file handed over by "Open with"; AppRoot routes it by type, then clears it. */
    private var openRequest by mutableStateOf<Uri?>(null)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        if (savedInstanceState == null) handle(intent)
        val privacy = PrivacySettings.get(this)
        lock.returning(privacy.lockApp)
        setContent {
            // No screenshots or recent-apps preview while "block screenshots" or the lock is on.
            val secure = privacy.blockScreenshots || privacy.lockApp
            DisposableEffect(secure) {
                if (secure) window.addFlags(WindowManager.LayoutParams.FLAG_SECURE) else window.clearFlags(WindowManager.LayoutParams.FLAG_SECURE)
                onDispose { }
            }
            MobileOfficeTheme {
                if (privacy.lockApp && lock.locked) {
                    LockScreen(onUnlock = ::askToUnlock)
                    LaunchedEffect(Unit) { askToUnlock() }
                } else {
                    AppRoot(sheet, word, pdf, slides, cad, image, text, archive, invoice, openRequest, onOpenHandled = { openRequest = null })
                }
            }
        }
    }

    private fun askToUnlock() {
        lock.ask(this, getString(R.string.lock_title), confirmCredential::launch)
    }

    override fun onStop() {
        super.onStop()
        lock.leaving()
    }

    override fun onStart() {
        super.onStart()
        lock.returning(PrivacySettings.get(this).lockApp)
    }

    /** Set while a viewer turns pages with the volume keys; called with +1 (next) or -1 (previous). */
    var volumeKeyHandler: ((Int) -> Unit)? = null

    override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean {
        val handler = volumeKeyHandler
        val step = volumeStep(keyCode)
        if (handler == null || step == 0) return super.onKeyDown(keyCode, event)
        handler(step)
        return true
    }

    // Consume the key-up too, so the volume does not change while pages turn.
    override fun onKeyUp(keyCode: Int, event: KeyEvent): Boolean =
        if (volumeKeyHandler != null && volumeStep(keyCode) != 0) true else super.onKeyUp(keyCode, event)

    private fun volumeStep(keyCode: Int) = when (keyCode) {
        KeyEvent.KEYCODE_VOLUME_DOWN -> 1
        KeyEvent.KEYCODE_VOLUME_UP -> -1
        else -> 0
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

private val processLock = AppLock()

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

/** Shown instead of everything else while the app is locked. */
@Composable
private fun LockScreen(onUnlock: () -> Unit) {
    Box(
        Modifier.fillMaxSize().background(MaterialTheme.colorScheme.surface),
        contentAlignment = Alignment.Center,
    ) {
        Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(16.dp)) {
            Text("🔒", style = MaterialTheme.typography.displayMedium)
            Text(stringResource(R.string.lock_body), style = MaterialTheme.typography.bodyLarge)
            Button(onClick = onUnlock) { Text(stringResource(R.string.lock_unlock)) }
        }
    }
}
