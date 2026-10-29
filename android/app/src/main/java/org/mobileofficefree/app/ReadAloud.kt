package org.mobileofficefree.app

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.speech.tts.TextToSpeech
import android.speech.tts.UtteranceProgressListener
import java.util.Locale

/**
 * Reading pages aloud with the phone's own text-to-speech voices (Turkish when the phone has
 * one, else the phone's language). The app hands text to the system service; it never goes online
 * itself. [onPage] is told which page is being read, [onDone] when reading stops, [onProblem]
 * when it cannot start; all on the main thread.
 */
class ReadAloud(
    context: Context,
    private val onPage: (Int) -> Unit,
    private val onDone: () -> Unit,
    private val onProblem: (Problem) -> Unit,
) {
    enum class Problem { NoEngine, NoVoice }

    private val main = Handler(Looper.getMainLooper())
    private var ready = false
    private var queued: Pair<List<String>, Int>? = null
    private val tts: TextToSpeech = TextToSpeech(context.applicationContext) { status ->
        main.post { onInit(status) }
    }

    init {
        tts.setOnUtteranceProgressListener(object : UtteranceProgressListener() {
            override fun onStart(id: String) {
                id.substringBefore('-').toIntOrNull()?.let { page -> main.post { onPage(page) } }
            }

            override fun onDone(id: String) {
                if (id.endsWith(LAST)) main.post { this@ReadAloud.onDone() }
            }

            @Deprecated("Required by older APIs")
            override fun onError(id: String) {
                main.post { this@ReadAloud.onDone() }
            }
        })
    }

    private fun onInit(status: Int) {
        if (status != TextToSpeech.SUCCESS) {
            onProblem(Problem.NoEngine)
            return
        }
        val turkish = Locale.forLanguageTag("tr-TR")
        val lang = if (tts.isLanguageAvailable(turkish) >= TextToSpeech.LANG_AVAILABLE) turkish else Locale.getDefault()
        if (tts.setLanguage(lang) < TextToSpeech.LANG_AVAILABLE) {
            onProblem(Problem.NoVoice)
            return
        }
        ready = true
        queued?.let { (pages, from) -> speak(pages, from) }
        queued = null
    }

    /** Reads [pages] from page [from] to the end, replacing anything being read. */
    fun speak(pages: List<String>, from: Int) {
        if (!ready) {
            queued = pages to from
            return
        }
        tts.stop()
        val max = TextToSpeech.getMaxSpeechInputLength().coerceAtMost(3_000)
        var any = false
        for (page in from until pages.size) {
            for ((n, chunk) in chunks(pages[page], max).withIndex()) {
                tts.speak(chunk, TextToSpeech.QUEUE_ADD, null, "$page-$n")
                any = true
            }
        }
        if (any) tts.speak(" ", TextToSpeech.QUEUE_ADD, null, "end-$LAST") else onDone()
    }

    fun stop() {
        queued = null
        tts.stop()
    }

    fun shutdown() {
        tts.stop()
        tts.shutdown()
    }

    private companion object {
        const val LAST = "last"

        /** Pieces of at most [max] characters, cut after a sentence or line where possible. */
        fun chunks(text: String, max: Int): List<String> {
            val out = ArrayList<String>()
            var rest = text.trim()
            while (rest.length > max) {
                val window = rest.substring(0, max)
                val cut = maxOf(window.lastIndexOf(". "), window.lastIndexOf('\n'), window.lastIndexOf(' '))
                val at = if (cut > max / 2) cut + 1 else max
                out += rest.substring(0, at).trim()
                rest = rest.substring(at).trim()
            }
            if (rest.isNotEmpty()) out += rest
            return out
        }
    }
}
