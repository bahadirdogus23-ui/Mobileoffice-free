package org.mobileofficefree.app

import android.app.ActivityManager
import android.app.Application
import android.content.Context
import android.os.Build
import android.os.Debug
import android.os.SystemClock
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
    private const val MAX_TIMINGS = 50

    private val timings = ArrayDeque<String>()

    /** Remembers how long [what] took since [startedAt] (a [SystemClock.elapsedRealtime] value). */
    fun record(what: String, startedAt: Long) {
        val ms = SystemClock.elapsedRealtime() - startedAt
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
            runCatching {
                val trace = StringWriter().also { error.printStackTrace(PrintWriter(it)) }.toString()
                file.writeText(
                    buildString {
                        appendLine("${DateFormat.getDateTimeInstance().format(Date())} · ${thread.name}")
                        appendLine(device(context))
                        appendLine()
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
    }

    /** The engine library's size in the APK (it is kept uncompressed there, not unpacked). */
    private fun nativeLibrarySize(context: Context): Long? = runCatching {
        ZipFile(context.packageCodePath).use { zip ->
            Build.SUPPORTED_ABIS.firstNotNullOfOrNull { abi -> zip.getEntry("lib/$abi/libmo_ffi.so")?.size }
        }
    }.getOrNull() ?: File(context.applicationInfo.nativeLibraryDir, "libmo_ffi.so").takeIf { it.exists() }?.length()

    private fun mb(bytes: Long) = "%.1f MB".format(java.util.Locale.ROOT, bytes / (1024.0 * 1024.0))
}
