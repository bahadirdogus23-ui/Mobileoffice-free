package org.mobileofficefree.app

import android.app.KeyguardManager
import android.content.Context
import android.os.SystemClock
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.core.content.edit

/**
 * Privacy choices, kept across launches: lock the app with the phone's own lock (fingerprint,
 * face, PIN), block screenshots and the recent-apps preview, and open files without listing them
 * in "Recent". One instance for the process, so every screen sees a change at once.
 */
class PrivacySettings private constructor(context: Context) {
    private val prefs = context.applicationContext.getSharedPreferences("privacy", Context.MODE_PRIVATE)

    var lockApp by mutableStateOf(prefs.getBoolean(LOCK, false))
        private set
    var blockScreenshots by mutableStateOf(prefs.getBoolean(SECURE, false))
        private set
    var incognito by mutableStateOf(prefs.getBoolean(INCOGNITO, false))
        private set

    fun updateLockApp(on: Boolean) {
        lockApp = on
        prefs.edit { putBoolean(LOCK, on) }
    }

    fun updateBlockScreenshots(on: Boolean) {
        blockScreenshots = on
        prefs.edit { putBoolean(SECURE, on) }
    }

    fun updateIncognito(on: Boolean) {
        incognito = on
        prefs.edit { putBoolean(INCOGNITO, on) }
    }

    companion object {
        private const val LOCK = "lock_app"
        private const val SECURE = "block_screenshots"
        private const val INCOGNITO = "incognito"

        @Volatile private var instance: PrivacySettings? = null

        fun get(context: Context): PrivacySettings =
            instance ?: synchronized(this) { instance ?: PrivacySettings(context).also { instance = it } }
    }
}

/**
 * When the app should ask for the phone's lock again: on start, and after it has been in the
 * background for [GRACE_MS] (switching to a file picker or the camera for a moment does not count).
 */
class AppLock {
    var locked by mutableStateOf(true)
        private set
    private var leftAt = 0L

    fun leaving() {
        leftAt = SystemClock.elapsedRealtime()
    }

    fun returning(enabled: Boolean) {
        if (!enabled) {
            locked = false
        } else if (leftAt != 0L && SystemClock.elapsedRealtime() - leftAt > GRACE_MS) {
            locked = true
        }
    }

    fun unlocked() {
        locked = false
    }

    /** Whether the phone has a screen lock the app can ask for. */
    fun available(context: Context): Boolean = context.getSystemService(KeyguardManager::class.java)?.isDeviceSecure == true

    /**
     * Asks for the phone's lock through the system's own confirm-credential screen (PIN, pattern,
     * password, and the fingerprint or face where the phone offers them there). It needs no
     * permission, unlike the biometric prompt (AGENTS.md: no permissions beyond storage).
     * [launch] starts the screen; the activity reports success with [unlocked].
     */
    // Deprecated for the biometric prompt, which needs USE_BIOMETRIC; this screen needs nothing.
    @Suppress("DEPRECATION")
    fun ask(context: Context, title: String, launch: (android.content.Intent) -> Unit) {
        val intent = context.getSystemService(KeyguardManager::class.java)?.takeIf { it.isDeviceSecure }
            ?.createConfirmDeviceCredentialIntent(title, null)
        if (intent == null) locked = false else launch(intent)
    }

    private companion object {
        const val GRACE_MS = 30_000L
    }
}
