package org.mobileofficefree.app

import android.app.ActivityManager
import android.app.Application
import android.content.Context
import android.os.Build
import android.os.Debug
import android.os.SystemClock
import android.content.Intent
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.core.content.edit
import java.io.File
import java.io.PrintWriter
import java.io.StringWriter
import java.text.DateFormat
import java.util.Date
import java.util.zip.ZipFile

/** Starts crash recording before anything else runs. */
class MobileOfficeApp : Application() {
    override fun onCreate() {
        super.onCreate()
        Diagnostics.installCrashRecorder(this)
        // The screen on show, for a picture of it when something goes wrong.
        registerActivityLifecycleCallbacks(object : ActivityLifecycleCallbacks {
            override fun onActivityResumed(activity: android.app.Activity) { Diagnostics.current = java.lang.ref.WeakReference(activity) }
            override fun onActivityPaused(activity: android.app.Activity) {}
            override fun onActivityCreated(activity: android.app.Activity, state: android.os.Bundle?) {}
            override fun onActivityStarted(activity: android.app.Activity) {}
            override fun onActivityStopped(activity: android.app.Activity) {}
            override fun onActivitySaveInstanceState(activity: android.app.Activity, state: android.os.Bundle) {}
            override fun onActivityDestroyed(activity: android.app.Activity) {}
        })
    }
}

/**
 * What the person can send us without the app ever going online: a crash report written to the
 * app's own storage (offered for sharing on the next start) and timings of the documents opened
 * in this session, with the phone's model, memory and the app's size. Nothing leaves the phone
 * unless the person shares it themselves.
 */
object Diagnostics {
    private const val CRASH_FILE = "last-crash.txt"
    private const val CRASH_SHOT = "last-crash.png"
    private const val MAX_TIMINGS = 50
    private const val MAX_EVENTS = 80
    private const val MAX_SHOTS = 3

    /** Where problem reports go: the developer's mailbox, through the person's own mail app. */
    const val REPORT_EMAIL = "bahadirdogus@gmail.com"

    private val timings = ArrayDeque<String>()
    /** What happened in this session: problems, and what was opened, with times. */
    private val events = ArrayDeque<String>()
    @Volatile var current: java.lang.ref.WeakReference<android.app.Activity>? = null

    private fun now(): String = java.text.SimpleDateFormat("HH:mm:ss", java.util.Locale.ROOT).format(Date())

    /** Remembers something worth knowing in a report (an open, a save, a setting changed). */
    fun event(what: String) {
        synchronized(events) {
            events.addFirst("${now()} $what")
            while (events.size > MAX_EVENTS) events.removeLast()
        }
    }

    fun events(): List<String> = synchronized(events) { events.toList() }

    /**
     * Something went wrong that the person was told about: it is kept with the session's events,
     * and the screen as it is now is pictured for the report (unless screenshots are blocked).
     * Call on the main thread. Returns the picture, if one was taken.
     */
    fun problem(context: Context, what: String, error: Throwable? = null): File? {
        event("SORUN: $what" + (error?.let { " (${it.javaClass.simpleName}: ${it.message})" } ?: ""))
        return captureScreen(context)
    }

    /** The current screen as a PNG in the reports folder, unless the person blocks screenshots. */
    fun captureScreen(context: Context, name: String = "screen-${System.currentTimeMillis()}.png"): File? {
        val privacy = PrivacySettings.get(context)
        if (privacy.blockScreenshots || privacy.lockApp || !ReportSettings.get(context).screenshots) return null
        val activity = current?.get() ?: return null
        return runCatching {
            val view = activity.window.decorView
            if (view.width <= 0 || view.height <= 0) return null
            val bmp = android.graphics.Bitmap.createBitmap(view.width, view.height, android.graphics.Bitmap.Config.ARGB_8888)
            view.draw(android.graphics.Canvas(bmp))
            val dir = reportsDir(context)
            val file = File(dir, name)
            file.outputStream().use { bmp.compress(android.graphics.Bitmap.CompressFormat.PNG, 90, it) }
            bmp.recycle()
            // Only the newest few are kept.
            dir.listFiles { f -> f.name.startsWith("screen-") }?.sortedByDescending { it.lastModified() }?.drop(MAX_SHOTS)?.forEach { it.delete() }
            file
        }.getOrNull()
    }

    fun reportsDir(context: Context): File = File(context.cacheDir, "reports").apply { mkdirs() }

    /** The newest screen picture taken for a problem, if any. */
    fun latestScreen(context: Context): File? =
        reportsDir(context).listFiles { f -> f.name.startsWith("screen-") || f.name == CRASH_SHOT }?.maxByOrNull { it.lastModified() }

    /**
     * Opens the person's mail app with a report to [REPORT_EMAIL]: [summary], the full report
     * and the screen pictures as attachments. Nothing is sent until the person sends it; the app
     * itself has no internet access.
     */
    fun emailReport(context: Context, summary: String, shots: List<File> = listOfNotNull(latestScreen(context))) {
        val body = buildString {
            appendLine(summary)
            appendLine()
            appendLine("— Ne yapıyordunuz? (isteğe bağlı) —")
            appendLine()
            appendLine()
            append(report(context))
        }
        val uris = ArrayList(
            shots.filter { it.exists() }.mapNotNull { f ->
                runCatching { androidx.core.content.FileProvider.getUriForFile(context, "${context.packageName}.files", f) }.getOrNull()
            },
        )
        val subject = "MobileOffice hata bildirimi · ${appVersion(context)} · ${Build.MODEL}"
        val send = Intent(if (uris.size > 1) Intent.ACTION_SEND_MULTIPLE else Intent.ACTION_SEND)
            .setType(if (uris.isEmpty()) "text/plain" else "image/png")
            .putExtra(Intent.EXTRA_EMAIL, arrayOf(REPORT_EMAIL))
            .putExtra(Intent.EXTRA_SUBJECT, subject)
            .putExtra(Intent.EXTRA_TEXT, body)
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        when (uris.size) {
            0 -> {}
            1 -> send.putExtra(Intent.EXTRA_STREAM, uris[0])
            else -> send.putParcelableArrayListExtra(Intent.EXTRA_STREAM, uris)
        }
        // Mail apps only, when there is one; any app that takes text otherwise.
        send.selector = Intent(Intent.ACTION_SENDTO, android.net.Uri.parse("mailto:"))
        val chooser = Intent.createChooser(send, context.getString(R.string.report_send)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        try {
            context.startActivity(chooser)
        } catch (_: android.content.ActivityNotFoundException) {
            send.selector = null
            context.startActivity(Intent.createChooser(send, context.getString(R.string.report_send)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        }
    }

    private fun appVersion(context: Context): String =
        runCatching { context.packageManager.getPackageInfo(context.packageName, 0).versionName }.getOrNull().orEmpty()

    /** Remembers how long [what] took since [startedAt] (a [SystemClock.elapsedRealtime] value). */
    fun record(what: String, startedAt: Long) {
        val ms = SystemClock.elapsedRealtime() - startedAt
        event("$what: $ms ms")
        synchronized(timings) {
            timings.addFirst("$what: $ms ms")
            while (timings.size > MAX_TIMINGS) timings.removeLast()
        }
    }

    fun timings(): List<String> = synchronized(timings) { timings.toList() }

    fun installCrashRecorder(context: Context) {
        val file = File(context.filesDir, CRASH_FILE)
        val previous = Thread.getDefaultUncaughtExceptionHandler()
        Thread.setDefaultUncaughtExceptionHandler { thread, error ->
            // A crash on the screen's own thread: picture the screen as it was.
            if (thread == android.os.Looper.getMainLooper().thread) runCatching { captureScreen(context, CRASH_SHOT) }
            runCatching {
                val trace = StringWriter().also { error.printStackTrace(PrintWriter(it)) }.toString()
                file.writeText(
                    buildString {
                        appendLine("${DateFormat.getDateTimeInstance().format(Date())} · ${thread.name}")
                        appendLine(device(context))
                        appendLine()
                        appendLine("Son olaylar:")
                        events().take(20).forEach { appendLine("  $it") }
                        appendLine("Son işlemler:")
                        timings().take(10).forEach { appendLine("  $it") }
                        appendLine()
                        append(trace.take(20_000))
                    },
                )
            }
            previous?.uncaughtException(thread, error)
        }
    }

    /** The report of the last crash, if the app crashed since it was last read. */
    fun takeCrashReport(context: Context): String? {
        val file = File(context.filesDir, CRASH_FILE)
        if (!file.exists()) return null
        return runCatching { file.readText() }.getOrNull().also { file.delete() }
    }

    fun device(context: Context): String {
        val pkg = runCatching { context.packageManager.getPackageInfo(context.packageName, 0) }.getOrNull()
        return "${Build.MANUFACTURER} ${Build.MODEL} · Android ${Build.VERSION.RELEASE} (API ${Build.VERSION.SDK_INT}) · " +
            "${Build.SUPPORTED_ABIS.firstOrNull()} · uygulama ${pkg?.versionName}"
    }

    /** Everything on the measurement screen, as text to share. */
    fun report(context: Context): String = buildString {
        appendLine(device(context))
        val dm = context.resources.displayMetrics
        appendLine("Ekran: ${dm.widthPixels}×${dm.heightPixels} px, ${dm.densityDpi} dpi, yazı ölçeği ${context.resources.configuration.fontScale}")
        appendLine("Dil: ${java.util.Locale.getDefault().toLanguageTag()} · Saat dilimi: ${java.util.TimeZone.getDefault().id}")
        val theme = ThemeSettings.get(context)
        val reading = runCatching { ReadingSettings(context) }.getOrNull()
        appendLine("Tema: ${theme.look} / ${theme.mode} · Gece modu: ${reading?.night} · Koyu sayfalar: ${reading?.darkPages}")
        val privacy = PrivacySettings.get(context)
        appendLine("Kilit: ${privacy.lockApp} · Ekran görüntüsü engeli: ${privacy.blockScreenshots} · Gizli mod: ${privacy.incognito}")
        appendLine("Tüm dosyalara erişim: ${DeviceFiles.hasAccess(context)}")
        runCatching {
            val stat = android.os.StatFs(context.filesDir.path)
            appendLine("Depolama: boş ${mb(stat.availableBytes)} / ${mb(stat.totalBytes)}")
        }
        appendLine("Çalışma süresi: ${SystemClock.elapsedRealtime() / 60_000} dk (telefon)")
        val apk = File(context.packageCodePath)
        appendLine("APK: ${mb(apk.length())}")
        nativeLibrarySize(context)?.let { appendLine("Motor kitaplığı (libmo_ffi.so): ${mb(it)}") }
        val rt = Runtime.getRuntime()
        appendLine("Java belleği: ${mb(rt.totalMemory() - rt.freeMemory())} / ${mb(rt.maxMemory())}")
        appendLine("Yerel bellek: ${mb(Debug.getNativeHeapAllocatedSize())}")
        val am = context.getSystemService(ActivityManager::class.java)
        val pss = am?.getProcessMemoryInfo(intArrayOf(android.os.Process.myPid()))?.firstOrNull()?.totalPss
        if (pss != null) appendLine("Toplam bellek (PSS): ${mb(pss * 1024L)}")
        val info = ActivityManager.MemoryInfo().also { am?.getMemoryInfo(it) }
        appendLine("Telefonun belleği: ${mb(info.totalMem)}, boş ${mb(info.availMem)}")
        appendLine()
        appendLine("Açma süreleri (bu oturum):")
        val t = timings()
        if (t.isEmpty()) appendLine("  —") else t.forEach { appendLine("  $it") }
        appendLine()
        appendLine("Olaylar (bu oturum, en yenisi üstte):")
        val e = events()
        if (e.isEmpty()) appendLine("  —") else e.forEach { appendLine("  $it") }
    }

    /** The engine library's size in the APK (it is kept uncompressed there, not unpacked). */
    private fun nativeLibrarySize(context: Context): Long? = runCatching {
        ZipFile(context.packageCodePath).use { zip ->
            Build.SUPPORTED_ABIS.firstNotNullOfOrNull { abi -> zip.getEntry("lib/$abi/libmo_ffi.so")?.size }
        }
    }.getOrNull() ?: File(context.applicationInfo.nativeLibraryDir, "libmo_ffi.so").takeIf { it.exists() }?.length()

    private fun mb(bytes: Long) = "%.1f MB".format(java.util.Locale.ROOT, bytes / (1024.0 * 1024.0))
}

/** Whether problem reports carry a picture of the screen (Settings → Support). */
class ReportSettings private constructor(context: Context) {
    private val prefs = context.applicationContext.getSharedPreferences("reports", Context.MODE_PRIVATE)
    var screenshots by androidx.compose.runtime.mutableStateOf(prefs.getBoolean("screenshots", true))
        private set

    fun updateScreenshots(on: Boolean) {
        screenshots = on
        prefs.edit { putBoolean("screenshots", on) }
    }

    companion object {
        @Volatile private var instance: ReportSettings? = null
        fun get(context: Context): ReportSettings = instance ?: synchronized(this) { instance ?: ReportSettings(context).also { instance = it } }
    }
}
