package org.mobileofficefree.app

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.core.content.edit

/**
 * "My details": what the person tells the app about themselves, kept only on the phone. Their
 * name signs the notes they add (in place of an engine's placeholder author).
 */
class Profile private constructor(context: Context) {
    private val prefs = context.applicationContext.getSharedPreferences("profile", Context.MODE_PRIVATE)
    var name by mutableStateOf(prefs.getString("name", "").orEmpty())
        private set

    fun updateName(value: String) {
        name = value.trim()
        prefs.edit { putString("name", name) }
    }

    companion object {
        @Volatile private var instance: Profile? = null
        fun get(context: Context): Profile = instance ?: synchronized(this) { instance ?: Profile(context).also { instance = it } }
    }
}
