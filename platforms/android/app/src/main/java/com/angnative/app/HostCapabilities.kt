/**
 * HostCapabilities.kt — host-side async capability bridge for the QuickJS shims.
 *
 * JavaScript queues host work through `na_fetchAsyncStart(id, requestJson)`,
 * `na_timerSchedule(id, delayMs, intervalFlag, generation)` and
 * `na_timerCancel(id, generation)`. Rust exposes those queued tasks as a JSON
 * array (optionally wrapped in `{"tasks":[...]}`) via
 * `angnative_runtime_bridge_take_host_tasks`; JniBridge marshals that call onto
 * the owner/main thread. This class drains the queue, performs the work and
 * invokes `na_onFetchResult` / `na_onTimerTick` back in the JS runtime.
 *
 * No market data is fetched from the network here: `/marketpulse/quote` is
 * served from the socket-derived MarketPulseFeed payload. All native calls are
 * made on the main looper; worker threads only build response strings.
 */
package com.angnative.app

import android.os.Handler
import android.os.Looper
import android.util.Log
import org.json.JSONArray
import org.json.JSONObject
import java.util.Locale
import java.util.concurrent.Executors

class HostCapabilities(private val jniBridge: JniBridge) {

  private val mainLooper = Looper.getMainLooper()
  private val mainHandler = Handler(mainLooper)
  private val workExecutor = Executors.newSingleThreadExecutor { runnable ->
    Thread(runnable, "angnative-host-capabilities").apply { isDaemon = true }
  }

  /** id -> the single live tick runnable for that timer generation. */
  private val timers = mutableMapOf<Long, TimerTask>()
  private var cancelled = false

  companion object {
    private const val TAG = "AngNative"
    private const val HEADERS_JSON = "{\"content-type\":\"application/json\"}"
    /** Bound a single drain so a chatty JS callback cannot starve the looper. */
    private const val MAX_DRAIN_ROUNDS = 64

    private const val KIND_FETCH = "fetch"
    private const val KIND_TIMER_SCHEDULE = "timerSchedule"
    private const val KIND_TIMER_CANCEL = "timerCancel"
  }

  /**
   * Drain every currently queued capability task. Safe to call from the main
   * thread or a worker: [JniBridge.takeHostTasks] always marshals the native
   * call to the main looper. New tasks queued by a JS callback are picked up by
   * the next round (or by the next drain after a native event dispatch).
   */
  fun drain() {
    if (cancelled) return
    var round = 0
    while (!cancelled && round++ < MAX_DRAIN_ROUNDS) {
      val json = jniBridge.takeHostTasks() ?: return
      if (json.isBlank() || json.trim() == "null") return
      val tasks = parseTasks(json)
      if (tasks.isEmpty()) return
      for (task in tasks) dispatch(task)
    }
  }

  /** Cancel all timers and drop queued work. Called from AngNativeView.destroy. */
  fun cancelAll() {
    cancelled = true
    workExecutor.shutdownNow()
    if (Looper.myLooper() === mainLooper) {
      removeTimersNow()
    } else {
      mainHandler.post { removeTimersNow() }
    }
  }

  /* =======================================================================
   * Task decoding — accept camelCase/snake_case and string/number ids so the
   * Kotlin side does not have to mirror every Rust serde attribute.
   * ======================================================================= */

  private fun parseTasks(json: String): List<JSONObject> {
    val tasks = mutableListOf<JSONObject>()
    try {
      val text = json.trim()
      if (text.startsWith("[")) {
        val array = JSONArray(text)
        for (i in 0 until array.length()) {
          if (array.opt(i) is JSONObject) tasks.add(array.getJSONObject(i))
        }
      } else {
        val root = JSONObject(text)
        val array = root.optJSONArray("tasks") ?: root.optJSONArray("hostTasks")
        if (array != null) {
          for (i in 0 until array.length()) {
            if (array.opt(i) is JSONObject) tasks.add(array.getJSONObject(i))
          }
        } else {
          tasks.add(root)
        }
      }
    } catch (e: Exception) {
      Log.w(TAG, "takeHostTasks returned invalid JSON: ${e.message}")
    }
    return tasks
  }

  private fun taskKind(task: JSONObject): String? {
    val raw = stringValue(task, "kind", "type", "op", "name")
      ?.lowercase(Locale.US)
      ?: return null
    return when {
      raw.contains("fetch") -> KIND_FETCH
      raw.contains("cancel") && raw.contains("timer") -> KIND_TIMER_CANCEL
      raw.contains("interval") && raw.contains("cancel") -> KIND_TIMER_CANCEL
      raw.contains("timer") || raw.contains("interval") -> KIND_TIMER_SCHEDULE
      else -> null
    }
  }

  private fun dispatch(task: JSONObject) {
    if (cancelled) return
    when (taskKind(task)) {
      KIND_FETCH -> dispatchFetch(task)
      KIND_TIMER_SCHEDULE -> {
        val id = longValue(task, "id", "timerId")
        val generation = longValue(task, "generation", "gen")
        val delayMs = longValue(task, "delayMs", "delay_ms", "delay")
        val interval = boolValue(task, "intervalFlag", "interval_flag", "interval", "repeat")
        mainHandler.post { scheduleTimer(id, delayMs, interval, generation) }
      }
      KIND_TIMER_CANCEL -> {
        val id = longValue(task, "id", "timerId")
        val generation = longValue(task, "generation", "gen", default = -1L)
        mainHandler.post { cancelTimer(id, generation) }
      }
      else -> Log.w(TAG, "Unknown host capability task: ${task.toString().take(160)}")
    }
  }

  private fun dispatchFetch(task: JSONObject) {
    val id = longValue(task, "id", "requestId", "fetchId")
    var requestJson = when (val raw = value(task, "requestJson", "request", "req", "url")) {
      is JSONObject -> raw.toString()
      is String -> raw
      else -> raw?.toString().orEmpty()
    }
    if (requestJson.isBlank()) {
      stringValue(task, "url")?.let { requestJson = JSONObject().put("url", it).toString() }
    }
    workExecutor.execute {
      if (cancelled) return@execute
      val result = try {
        fetchResult(requestJson)
      } catch (e: Exception) {
        FetchResult(
          502,
          JSONObject().put("error", "${e.javaClass.simpleName}: ${e.message ?: "no message"}").toString(),
          HEADERS_JSON,
        )
      }
      val script = "na_onFetchResult($id, ${result.status}, " +
        "${JSONObject.quote(result.bodyText)}, ${JSONObject.quote(result.headersJson)}); 0;"
      callScript(script)
    }
  }

  private fun fetchResult(requestJson: String): FetchResult {
    val url = try {
      val trimmed = requestJson.trim()
      if (trimmed.startsWith("{")) JSONObject(trimmed).optString("url") else trimmed
    } catch (_: Exception) {
      requestJson
    }
    // Socket-only market data: the host never opens a REST connection here.
    return if (url.contains("/marketpulse/quote")) {
      FetchResult(200, MarketPulseFeed.latestPayload(), HEADERS_JSON)
    } else {
      FetchResult(502, "{\"error\":\"unsupported fetch url\"}", HEADERS_JSON)
    }
  }

  /* =======================================================================
   * Timers — one runnable per id/generation, coalesced through the main looper.
   * ======================================================================= */

  private inner class TimerTask(
    val id: Long,
    val generation: Long,
    val delayMs: Long,
    val interval: Boolean,
  ) : Runnable {
    override fun run() {
      if (cancelled || timers[id] !== this) return
      // Already on the main looper: the JS tick is a native loadScript call.
      executeScript("na_onTimerTick($id, $generation); 0;")
      if (cancelled || timers[id] !== this) return
      if (interval) {
        mainHandler.postDelayed(this, maxOf(1L, delayMs))
      } else {
        timers.remove(id)
      }
    }
  }

  private fun scheduleTimer(id: Long, delayMs: Long, interval: Boolean, generation: Long) {
    if (cancelled) return
    cancelTimer(id, -1L) // replace any older generation/runnable for this id
    val task = TimerTask(id, generation, delayMs, interval)
    timers[id] = task
    mainHandler.postDelayed(task, maxOf(0L, delayMs))
  }

  private fun cancelTimer(id: Long, generation: Long) {
    if (cancelled) return
    val existing = timers[id] ?: return
    // The JS shim increments the generation BEFORE queueing the cancel, so the
    // queued value is the replacement generation. Any live runnable for this
    // id is therefore stale and must be removed; the JS generation check then
    // suppresses any tick already in flight.
    if (generation >= 0L && generation != existing.generation) {
      Log.d(TAG, "Cancelled stale timer $id (host generation ${existing.generation})")
    }
    mainHandler.removeCallbacks(existing)
    timers.remove(id)
  }

  private fun removeTimersNow() {
    for (task in timers.values) mainHandler.removeCallbacks(task)
    timers.clear()
  }

  /* =======================================================================
   * Native calls (main thread only).
   * ======================================================================= */

  private fun callScript(script: String) {
    if (Looper.myLooper() === mainLooper) {
      executeScript(script)
    } else {
      mainHandler.post { executeScript(script) }
    }
  }

  private fun executeScript(script: String) {
    if (cancelled) return
    val rc = jniBridge.loadScript(script)
    if (rc != 0) {
      Log.e(TAG, "Host capability callback failed (rc=$rc): ${jniBridge.lastErrorJson()}")
      return
    }
    // A callback can queue more host work (a timer tick asks for a fresh
    // quote, for example). Pick that up on the next looper turn.
    if (!cancelled) mainHandler.post { drain() }
  }

  private data class FetchResult(val status: Int, val bodyText: String, val headersJson: String)

  private fun value(obj: JSONObject, vararg names: String): Any? {
    for (name in names) {
      val found = obj.opt(name)
      if (found != null && found != JSONObject.NULL) return found
    }
    // Some producers wrap task arguments in `payload`/`data`.
    val nested = obj.optJSONObject("payload") ?: obj.optJSONObject("data")
    if (nested != null) {
      for (name in names) {
        val found = nested.opt(name)
        if (found != null && found != JSONObject.NULL) return found
      }
    }
    return null
  }

  private fun stringValue(obj: JSONObject, vararg names: String): String? =
    value(obj, *names)?.toString()

  private fun longValue(obj: JSONObject, vararg names: String, default: Long = 0L): Long {
    val raw = stringValue(obj, *names) ?: return default
    return raw.toLongOrNull() ?: raw.toDoubleOrNull()?.toLong() ?: default
  }

  private fun boolValue(obj: JSONObject, vararg names: String): Boolean {
    val raw = stringValue(obj, *names)?.lowercase(Locale.US) ?: return false
    return raw == "true" || raw == "1"
  }
}
