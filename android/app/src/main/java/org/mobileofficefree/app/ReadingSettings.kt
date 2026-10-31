package org.mobileofficefree.app

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.core.content.edit

/** The "Okuma" (reading) choices, shared by every paged viewer and kept across launches. */
class ReadingSettings(context: Context) {
    private val prefs = context.applicationContext.getSharedPreferences("reading", Context.MODE_PRIVATE)

    var night by mutableStateOf(prefs.getBoolean(NIGHT, false))
        private set
    var keepScreenOn by mutableStateOf(prefs.getBoolean(KEEP_ON, false))
        private set
    var volumeKeys by mutableStateOf(prefs.getBoolean(VOLUME_KEYS, false))
        private set
    /** Text size of the reflowing text view, in sp. */
    var textSize by mutableFloatStateOf(prefs.getFloat(TEXT_SIZE, 18f))
        private set

    fun updateTextSize(sp: Float) {
        textSize = sp.coerceIn(12f, 40f)
        prefs.edit { putFloat(TEXT_SIZE, textSize) }
    }

    fun updateNight(on: Boolean) {
        night = on
        prefs.edit { putBoolean(NIGHT, on) }
    }

    fun updateKeepScreenOn(on: Boolean) {
        keepScreenOn = on
        prefs.edit { putBoolean(KEEP_ON, on) }
    }

    fun updateVolumeKeys(on: Boolean) {
        volumeKeys = on
        prefs.edit { putBoolean(VOLUME_KEYS, on) }
    }

    private companion object {
        const val NIGHT = "night"
        const val KEEP_ON = "keep_screen_on"
        const val VOLUME_KEYS = "volume_keys"
        const val TEXT_SIZE = "text_size"
    }
}
