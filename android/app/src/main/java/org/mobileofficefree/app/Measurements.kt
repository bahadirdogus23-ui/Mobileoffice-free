package org.mobileofficefree.app

import android.content.ComponentCallbacks2
import android.content.Context
import android.os.Build
import android.os.Debug
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.Choreographer
import java.io.File

/**
 * The detailed side of Settings › Measurements, kept on the phone across sessions until cleared:
 *
 * - **Durations** of everything the app does again and again (drawing a page, an edit, a save,
 *   a recovery copy…), each as count, average, fastest, slowest, last and how many were slow.
 * - **Stutters**: frames the screen waited for (over 50 ms), counted per screen; long ones
 *   (over 250 ms) are kept as events with what was on screen.
 * - **Memory** sampled every minute while the app is in use, and Android's low-memory calls.
 *
 * Nothing leaves the phone unless the person sends the report.
 */
object Measurements {
    private const val FILE = "measurements-stats.txt"
    /** Over this an operation counts as slow. */
    private const val SLOW_MS = 500L
    /** A frame that took longer than this is a stutter; longer than [LONG_STUTTER_MS], an event. */
    private const val STUTTER_MS = 50L
    private const val LONG_STUTTER_MS = 250L

    class Stat(var count: Long = 0, var total: Long = 0, var min: Long = Long.MAX_VALUE, var max: Long = 0, var last: Long = 0, var slow: Long = 0)

    private val stats = LinkedHashMap<String, Stat>()
    private val stutters = LinkedHashMap<String, LongArray>() // screen → [frames, stutters, long stutters, worst ms]
    @Volatile private var file: File? = null
    private val writer = java.util.concurrent.Executors.newSingleThreadExecutor()
    private var dirty = false

    /** What is on screen now ("Ana ekran", "PDF", "Excel"…), for stutters and events. */
    @Volatile var screen: String = "Ana ekran"

    /** When the process started (for the cold start time). */
    private val processStart = SystemClock.elapsedRealtime()
    @Volatile private var firstFrameMs = -1L

    /** [clear] was called before the kept history was loaded: it is not brought back. Guarded by this object. */
    private var clearedBeforeLoad = false

    /**
     * Reads the durations and stutters kept from earlier sessions, on the writer's thread (this
     * runs in `Application.onCreate`). What is measured meanwhile is added to them, and nothing is
     * written until they are in memory ([file] stays null until then), so the file is never
     * replaced without them.
     */
    fun load(context: Context) {
        val f = File(context.filesDir, FILE)
        writer.execute {
            val keptStats = LinkedHashMap<String, Stat>()
            val keptStutters = LinkedHashMap<String, LongArray>()
            runCatching {
                if (f.exists()) {
                    f.forEachLine { line ->
                        val record = parseMeasurementRecord(line) ?: return@forEachLine
                        val v = record.values
                        when (record.kind) {
                            "S" -> keptStats[record.name] = Stat(v[0], v[1], v[2], v[3], v[4], v.getOrElse(5) { 0 })
                            "J" -> keptStutters[record.name] = v.toLongArray()
                        }
                    }
                }
            }
            synchronized(Measurements) {
                if (!clearedBeforeLoad) {
                    synchronized(stats) {
                        for ((name, s) in stats) {
                            val kept = keptStats[name]
                            keptStats[name] = if (kept == null) s else Stat(kept.count + s.count, kept.total + s.total, minOf(kept.min, s.min), maxOf(kept.max, s.max), s.last, kept.slow + s.slow)
                        }
                        stats.clear()
                        stats.putAll(keptStats)
                    }
                    synchronized(stutters) {
                        for ((name, a) in stutters) {
                            val kept = keptStutters[name]
                            keptStutters[name] = if (kept == null || kept.size < 4) a else longArrayOf(kept[0] + a[0], kept[1] + a[1], kept[2] + a[2], maxOf(kept[3], a[3]))
                        }
                        stutters.clear()
                        stutters.putAll(keptStutters)
                    }
                }
                file = f
            }
            // What was measured (or cleared) while loading reaches the file now.
            flush()
        }
    }

    /** Remembers that [name] took [ms] milliseconds. */
    fun sample(name: String, ms: Long) {
        synchronized(stats) {
            val s = stats.getOrPut(name) { Stat() }
            s.count++
            s.total += ms
            s.min = minOf(s.min, ms)
            s.max = maxOf(s.max, ms)
            s.last = ms
            if (ms > SLOW_MS) s.slow++
        }
        if (ms > SLOW_MS * 4) Diagnostics.event("YAVAŞ: $name $ms ms ($screen)")
        markDirty()
    }

    /** Runs [block], remembering how long it took as [name]. */
    inline fun <T> time(name: String, block: () -> T): T {
        val t0 = SystemClock.elapsedRealtime()
        try {
            return block()
        } finally {
            sample(name, SystemClock.elapsedRealtime() - t0)
        }
    }

    @Synchronized fun clear() {
        synchronized(stats) { stats.clear() }
        synchronized(stutters) { stutters.clear() }
        if (file == null) clearedBeforeLoad = true
        dirty = true
        flush()
    }

    @Synchronized private fun markDirty() {
        // Written at most every few seconds, off the main thread.
        if (dirty) return
        dirty = true
        Handler(Looper.getMainLooper()).postDelayed({ flush() }, 5_000)
    }

    /** Writes the durations and stutters out now (also when the app leaves the screen). */
    @Synchronized fun flush() {
        val f = file ?: return
        if (!dirty) return
        dirty = false
        val lines = buildList {
            synchronized(stats) { stats.forEach { (k, s) -> add("S\t${k.replace('\t', ' ')}\t${s.count}\t${s.total}\t${s.min}\t${s.max}\t${s.last}\t${s.slow}") } }
            synchronized(stutters) { stutters.forEach { (k, a) -> add("J\t${k.replace('\t', ' ')}\t${a[0]}\t${a[1]}\t${a[2]}\t${a[3]}") } }
        }
        writer.execute {
            runCatching {
                writeMeasurementSnapshot(f, lines.joinToString("\n"))
            }.onFailure { handler.post { markDirty() } }
        }
    }

    // ---- Stutters -------------------------------------------------------------------------

    private var watching = false
    private var lastFrame = 0L

    private val frameCallback = object : Choreographer.FrameCallback {
        override fun doFrame(frameTimeNanos: Long) {
            if (!watching) return
            val now = frameTimeNanos / 1_000_000
            if (firstFrameMs < 0) {
                firstFrameMs = SystemClock.elapsedRealtime() - processStart
                Diagnostics.event("İlk kare: $firstFrameMs ms (uygulama başlangıcından)")
            }
            if (lastFrame != 0L) {
                val gap = now - lastFrame
                synchronized(stutters) {
                    val a = stutters.getOrPut(screen) { LongArray(4) }
                    a[0]++
                    if (gap > STUTTER_MS) a[1]++
                    if (gap > LONG_STUTTER_MS) a[2]++
                    if (gap > a[3]) a[3] = gap
                }
                // A frame the screen waited long for: kept with what was on screen. (Gaps over
                // 5 s are the app sleeping in the background, not a stutter.)
                if (gap in (LONG_STUTTER_MS + 1)..5_000) Diagnostics.event("Takılma: $gap ms ($screen)")
                if (gap > STUTTER_MS) markDirty()
            }
            lastFrame = now
            Choreographer.getInstance().postFrameCallback(this)
        }
    }

    /** Starts (or stops) counting frames; call on the main thread as the app comes to (or leaves) the screen. */
    fun watchFrames(on: Boolean) {
        if (on == watching) return
        watching = on
        lastFrame = 0L
        if (on) Choreographer.getInstance().postFrameCallback(frameCallback) else flush()
    }

    // ---- Memory ---------------------------------------------------------------------------

    private val handler = Handler(Looper.getMainLooper())
    private var memoryContext: Context? = null
    private val memoryTick = object : Runnable {
        override fun run() {
            val c = memoryContext ?: return
            sampleMemory(c)
            handler.postDelayed(this, 60_000)
        }
    }

    /** Samples memory every minute while the app is on screen. */
    fun watchMemory(context: Context, on: Boolean) {
        handler.removeCallbacks(memoryTick)
        memoryContext = if (on) context.applicationContext else null
        if (on) handler.postDelayed(memoryTick, 2_000)
    }

    private fun sampleMemory(context: Context) {
        val rt = Runtime.getRuntime()
        sampleValue("Bellek: Java (MB)", (rt.totalMemory() - rt.freeMemory()) / MB)
        sampleValue("Bellek: yerel (MB)", Debug.getNativeHeapAllocatedSize() / MB)
        val am = context.getSystemService(android.app.ActivityManager::class.java)
        am?.getProcessMemoryInfo(intArrayOf(android.os.Process.myPid()))?.firstOrNull()?.totalPss?.let { sampleValue("Bellek: toplam PSS (MB)", it / 1024L) }
    }

    /** A value (not a duration) kept the same way, min/avg/max. */
    private fun sampleValue(name: String, v: Long) {
        synchronized(stats) {
            val s = stats.getOrPut(name) { Stat() }
            s.count++
            s.total += v
            s.min = minOf(s.min, v)
            s.max = maxOf(s.max, v)
            s.last = v
        }
        markDirty()
    }

    /** Android asked the app to give memory back: worth knowing when pages went blank or slow. */
    fun trimmed(level: Int) {
        val what = when {
            level >= ComponentCallbacks2.TRIM_MEMORY_COMPLETE -> "çok az bellek (arka planda, kapatılabilir)"
            level >= ComponentCallbacks2.TRIM_MEMORY_MODERATE -> "az bellek (arka planda)"
            level >= ComponentCallbacks2.TRIM_MEMORY_BACKGROUND -> "arka plana geçti"
            level >= ComponentCallbacks2.TRIM_MEMORY_UI_HIDDEN -> return
            level >= ComponentCallbacks2.TRIM_MEMORY_RUNNING_CRITICAL -> "ÇALIŞIRKEN kritik bellek azlığı"
            level >= ComponentCallbacks2.TRIM_MEMORY_RUNNING_LOW -> "çalışırken bellek azlığı"
            else -> "bellek uyarısı ($level)"
        }
        Diagnostics.event("Android: $what · ekran: $screen")
    }

    // ---- Report ---------------------------------------------------------------------------

    /** The detailed sections of the report. */
    fun report(context: Context): String = buildString {
        appendLine("── Uygulama ──")
        val pkg = runCatching { context.packageManager.getPackageInfo(context.packageName, 0) }.getOrNull()
        if (pkg != null) {
            val code = if (Build.VERSION.SDK_INT >= 28) pkg.longVersionCode else @Suppress("DEPRECATION") pkg.versionCode.toLong()
            val df = java.text.DateFormat.getDateTimeInstance(java.text.DateFormat.SHORT, java.text.DateFormat.SHORT)
            appendLine("Sürüm: ${pkg.versionName} (kod $code) · kuruldu ${df.format(java.util.Date(pkg.firstInstallTime))} · güncellendi ${df.format(java.util.Date(pkg.lastUpdateTime))}")
        }
        appendLine("Bu oturum: ${(SystemClock.elapsedRealtime() - processStart) / 1000} sn açık · ilk kare ${if (firstFrameMs >= 0) "$firstFrameMs ms" else "—"}")
        appendLine("Ekran şimdi: $screen")
        appendLine()
        appendLine("── Cihaz ayrıntısı ──")
        appendLine("İşlemci: ${Runtime.getRuntime().availableProcessors()} çekirdek · ABI ${Build.SUPPORTED_ABIS.joinToString()}" + (if (Build.VERSION.SDK_INT >= 31) " · SoC ${Build.SOC_MANUFACTURER} ${Build.SOC_MODEL}" else ""))
        appendLine("Android güvenlik yaması: ${Build.VERSION.SECURITY_PATCH} · derleme ${Build.DISPLAY}")
        val display = (context.getSystemService(android.hardware.display.DisplayManager::class.java))?.getDisplay(android.view.Display.DEFAULT_DISPLAY)
        display?.let { appendLine("Yenileme hızı: ${"%.0f".format(java.util.Locale.ROOT, it.refreshRate)} Hz") }
        val pm = context.getSystemService(android.os.PowerManager::class.java)
        val thermal = if (Build.VERSION.SDK_INT >= 29) when (pm?.currentThermalStatus) {
            0 -> "normal"; 1 -> "hafif"; 2 -> "orta"; 3 -> "yüksek"; 4 -> "kritik"; 5 -> "acil"; 6 -> "kapanma"; else -> "?"
        } else "—"
        val battery = context.getSystemService(android.os.BatteryManager::class.java)?.getIntProperty(android.os.BatteryManager.BATTERY_PROPERTY_CAPACITY)
        appendLine("Pil: %${battery ?: "?"} · güç tasarrufu ${if (pm?.isPowerSaveMode == true) "AÇIK" else "kapalı"} · ısı durumu $thermal")
        val am = context.getSystemService(android.app.ActivityManager::class.java)
        appendLine("Bellek sınıfı: ${am?.memoryClass} MB (büyük: ${am?.largeMemoryClass} MB) · düşük bellekli cihaz: ${am?.isLowRamDevice}")
        appendLine()
        appendLine("── Depolama ──")
        appendLine("Önbellek: ${dirSize(context.cacheDir) / MB} MB · kurtarma kopyaları: ${dirSize(File(context.filesDir, "recovery")) / 1024} KB · uygulama verisi: ${dirSize(context.filesDir) / 1024} KB")
        appendLine()
        appendLine("── Süreler (ms; adet · ortalama · en hızlı · en yavaş · son · yavaş >${SLOW_MS}) ──")
        val rows = synchronized(stats) { stats.entries.map { (name, s) -> name to Stat(s.count, s.total, s.min, s.max, s.last, s.slow) } }
        if (rows.isEmpty()) appendLine("  —")
        for ((name, s) in rows.sortedBy { it.first }) {
            val avg = if (s.count > 0) s.total / s.count else 0
            val slow = if (name.startsWith("Bellek")) "" else " · yavaş ${s.slow}"
            appendLine("  $name: ${s.count} · ort $avg · min ${if (s.min == Long.MAX_VALUE) 0 else s.min} · maks ${s.max} · son ${s.last}$slow")
        }
        appendLine()
        appendLine("── Takılmalar (kare; sayılan · >${STUTTER_MS} ms · >${LONG_STUTTER_MS} ms · en uzun) ──")
        val js = synchronized(stutters) { stutters.entries.map { it.key to it.value.copyOf() } }
        if (js.isEmpty()) appendLine("  —")
        for ((name, a) in js.sortedByDescending { it.second[1] }) {
            val pct = if (a[0] > 0) "%.1f".format(java.util.Locale.ROOT, a[1] * 100.0 / a[0]) else "0"
            appendLine("  $name: ${a[0]} kare · ${a[1]} takılma (%$pct) · ${a[2]} uzun · en uzun ${a[3]} ms")
        }
    }

    private fun dirSize(dir: File): Long = runCatching { dir.walkTopDown().filter { it.isFile }.sumOf { it.length() } }.getOrDefault(0L)

    private const val MB = 1024L * 1024L
}

/** A short Turkish name for the measurements ("Word", "Excel"…). */
fun DocumentKind.label(): String = when (this) {
    DocumentKind.WORD -> "Word"
    DocumentKind.SHEET -> "Excel"
    DocumentKind.SLIDES -> "Sunu"
    DocumentKind.PDF -> "PDF"
    DocumentKind.CAD -> "CAD"
    DocumentKind.IMAGE -> "Resim"
    DocumentKind.TEXT -> "Metin"
    DocumentKind.ARCHIVE -> "Arşiv"
    DocumentKind.INVOICE -> "e-Fatura"
    DocumentKind.OTHER -> "Diğer"
}
