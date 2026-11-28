/**
 * JniBridge.kt — JNI bridge between Android and the Rust host (angnative-host-ffi C ABI).
 *
 * Loads libangnative.so and calls the C ABI functions:
 *   - angnative_host_create / angnative_host_destroy
 *   - angnative_host_register_view_factory
 *   - angnative_host_process_batch
 *   - angnative_host_run_layout
 *   - angnative_host_last_error
 *   - angnative_host_node_count / angnative_host_snapshot
 *
 * The C++ side owns a heap-allocated ViewFactoryContext containing a JNI global
 * ref to this instance. Rust-emitted PlatformOps are delivered to the `onNative*`
 * methods below, which delegate to the installed NativeViewSink.
 *
 * Threading: every native entry point is marshalled to Android's main looper.
 * Rust→Kotlin view callbacks fire synchronously inside those calls, so they
 * inherit the main-thread guarantee without per-callback latches.
 */

package com.angnative.app

import android.content.Context
import android.graphics.Color
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.Gravity
import android.view.View
import android.widget.*
import org.json.JSONArray
import org.json.JSONObject
import java.nio.ByteBuffer
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference

class JniBridge(private val context: Context) {

  companion object {
    private const val TAG = "AngNative"
    /** Node bound to the demo's output Text, updated by native presses. */
    private const val OUTPUT_NODE_ID = 6L
    /** Node bound to the demo's Reset button. */
    private const val RESET_NODE_ID = 7L
    /** Upper bound for a posted native call; failures are propagated, not ignored. */
    private const val NATIVE_CALL_TIMEOUT_MS = 10_000L
    /** Stable FFI ErrorCode::InvalidHandle, used by the safe lifecycle wrappers. */
    private const val FFI_INVALID_HANDLE = 4

    init {
      System.loadLibrary("angnative")
    }
  }

  // Native methods loaded from libangnative.so
  // NOTE: nativeHostCreate/nativeHostDestroy were removed — the runtime bridge
  // owns the Host. Calling a standalone host destroy on a bridge-owned handle
  // would double-free it. Use nativeRuntimeBridge* exclusively.
  external fun nativeHostRegisterViewFactory(handle: Long)
  external fun nativeHostProcessBatch(handle: Long, batchJson: String): String?
  /** Borrow a caller-owned direct ByteBuffer for one synchronous call; no pointer is retained. */
  external fun nativeHostProcessBatchBytes(handle: Long, buffer: ByteBuffer): String?
  external fun nativeHostRunLayout(handle: Long, width: Float, height: Float): String?
  external fun nativeHostLastError(handle: Long): String?
  external fun nativeHostNodeCount(handle: Long): Int
  external fun nativeHostSnapshot(handle: Long): String?
  
  // Runtime bridge methods (unified host + QuickJS runtime)
  external fun nativeRuntimeBridgeCreate(): Long
  external fun nativeRuntimeBridgeDestroy(handle: Long, hostHandle: Long)
  external fun nativeRuntimeBridgeDispatchEvent(handle: Long, nodeId: Long, eventName: String, payloadJson: String?): Int
  external fun nativeRuntimeBridgeGetHost(handle: Long): Long
  external fun nativeRuntimeBridgeLoadScript(handle: Long, source: String): Int
  /** Reload one module in the existing JS realm; local state is preserved. */
  external fun nativeRuntimeBridgeHmrReload(handle: Long, moduleId: String, source: String): Int
  external fun nativeRuntimeBridgeLifecycleState(handle: Long): Int
  external fun nativeRuntimeBridgeSuspend(handle: Long): Int
  external fun nativeRuntimeBridgeResume(handle: Long): Int
  external fun nativeRuntimeBridgeStop(handle: Long): Int
  external fun nativeRuntimeBridgeMetricsJson(handle: Long): String?
  external fun nativeRuntimeBridgeLastErrorJson(handle: Long): String?
  external fun nativeRuntimeBridgeTakeHostTasks(handle: Long): String?

  private var hostHandle: Long = 0
  private var runtimeBridgeHandle: Long = 0
  private var isInitialized: Boolean = false
  private var pressCount: Int = 0

  private val mainLooper: Looper = Looper.getMainLooper()
  private val mainHandler = Handler(mainLooper)

  /** Installed by the platform view; receives the Rust ViewFactory callbacks. */
  var viewFactory: NativeViewSink? = null

  /**
   * Run a native call on Android's main looper.
   *
   * The inline fast path preserves synchronous shell code such as
   * `findButtonIdByText("+")` immediately after `loadScript`. Off-main callers
   * post once and wait; a timeout or captured exception becomes an
   * [IllegalStateException] so the caller never observes a false success.
   */
  private fun <T> runOnMainForNative(operation: () -> T): T {
    if (Looper.myLooper() === mainLooper) return operation()

    val result = AtomicReference<Any?>()
    val error = AtomicReference<Throwable>()
    val done = CountDownLatch(1)
    val posted = mainHandler.post {
      try {
        result.set(operation())
      } catch (t: Throwable) {
        error.set(t)
        Log.e(TAG, "Native bridge call failed on main thread", t)
      } finally {
        done.countDown()
      }
    }
    if (!posted) {
      throw IllegalStateException("Could not post native bridge call to main looper")
    }
    if (!done.await(NATIVE_CALL_TIMEOUT_MS, TimeUnit.MILLISECONDS)) {
      throw IllegalStateException("Native bridge call timed out after ${NATIVE_CALL_TIMEOUT_MS}ms")
    }
    error.get()?.let { throw IllegalStateException("Native bridge call failed on main thread", it) }
    @Suppress("UNCHECKED_CAST")
    return result.get() as T
  }

  fun initHost() = runOnMainForNative {
    if (isInitialized) return@runOnMainForNative

    // Create unified runtime bridge (host + QuickJS runtime). Running on main
    // keeps Rust's owner_thread aligned with later UI taps.
    runtimeBridgeHandle = nativeRuntimeBridgeCreate()
    if (runtimeBridgeHandle == 0L) {
      throw RuntimeException("Failed to create AngNative runtime bridge")
    }

    // Get the underlying host for ViewFactory registration
    hostHandle = nativeRuntimeBridgeGetHost(runtimeBridgeHandle)
    if (hostHandle == 0L) {
      // No host means no ViewFactory context was registered. Destroy the bridge
      // now so a failed init never leaks the u64 handle or its Rust owner.
      nativeRuntimeBridgeDestroy(runtimeBridgeHandle, 0L)
      runtimeBridgeHandle = 0L
      throw RuntimeException("Failed to get host from runtime bridge")
    }

    // C++ allocates a real ViewFactoryContext (JNI global ref) from `this`.
    // On any failure C++ releases its context before throwing; passing the
    // hostHandle below still lets C++ clean up a partially registered context
    // exactly once, with no double free.
    try {
      nativeHostRegisterViewFactory(hostHandle)
    } catch (t: Throwable) {
      nativeRuntimeBridgeDestroy(runtimeBridgeHandle, hostHandle)
      runtimeBridgeHandle = 0L
      hostHandle = 0L
      isInitialized = false
      throw t
    }
    isInitialized = true
  }

  fun initHostWithCallbacks() {
    initHost()
  }

  /* =======================================================================
   * Runtime lifecycle / diagnostics (main-thread marshalled, handle-guarded).
   * A zero/stale handle can only reach the explicit native* externals above;
   * these wrappers never call native with an uninitialized bridge.
   * ======================================================================= */

  /** -1 invalid/wrong thread, 0 Created … 3 Running … 7 Failed. */
  fun lifecycleState(): Int = runOnMainForNative {
    if (runtimeBridgeHandle == 0L) -1
    else nativeRuntimeBridgeLifecycleState(runtimeBridgeHandle)
  }

  /** Returns 0 on success or a stable FFI error code. */
  fun suspendRuntime(): Int = runOnMainForNative {
    if (runtimeBridgeHandle == 0L) FFI_INVALID_HANDLE
    else nativeRuntimeBridgeSuspend(runtimeBridgeHandle)
  }

  /** Returns 0 on success or a stable FFI error code. */
  fun resumeRuntime(): Int = runOnMainForNative {
    if (runtimeBridgeHandle == 0L) FFI_INVALID_HANDLE
    else nativeRuntimeBridgeResume(runtimeBridgeHandle)
  }

  /** Returns 0 on success or a stable FFI error code. */
  fun stopRuntime(): Int = runOnMainForNative {
    if (runtimeBridgeHandle == 0L) FFI_INVALID_HANDLE
    else nativeRuntimeBridgeStop(runtimeBridgeHandle)
  }

  /** Structured runtime counters as JSON (freed by the JNI layer). */
  fun metricsJson(): String = runOnMainForNative<String?> {
    if (runtimeBridgeHandle == 0L) null
    else nativeRuntimeBridgeMetricsJson(runtimeBridgeHandle)
  } ?: "{\"error\":\"metrics unavailable\"}"

  /** Last structured runtime error as JSON (freed by the JNI layer). */
  fun lastErrorJson(): String = runOnMainForNative<String?> {
    if (runtimeBridgeHandle == 0L) null
    else nativeRuntimeBridgeLastErrorJson(runtimeBridgeHandle)
  } ?: "{\"code\":$FFI_INVALID_HANDLE,\"category\":\"invalid_handle\"," +
    "\"phase\":\"none\",\"message\":\"runtime bridge not initialized\",\"stack\":null}"

  /**
   * Drain host-capability tasks queued by the QuickJS shims. Always called on
   * the owner/main thread through [runOnMainForNative]; a null result means
   * the queue is empty (or the bridge is gone).
   */
  fun takeHostTasks(): String? = runOnMainForNative<String?> {
    if (runtimeBridgeHandle == 0L) null
    else nativeRuntimeBridgeTakeHostTasks(runtimeBridgeHandle)
  }

  /** Load a self-contained JS bundle (e.g. compiled Angular AOT output). */
  fun loadScript(source: String): Int = runOnMainForNative {
    check(isInitialized) { "AngNative host not initialized" }
    nativeRuntimeBridgeLoadScript(runtimeBridgeHandle, source)
  }

  /** Reload one module in place; local state survives via the HMR contract. */
  fun hmrReload(moduleId: String, source: String): Int = runOnMainForNative {
    check(isInitialized) { "AngNative host not initialized" }
    nativeRuntimeBridgeHmrReload(runtimeBridgeHandle, moduleId, source)
  }

  /** Process a JSON command batch through the Rust host. */
  fun processBatch(batchJson: String): String {
    val result = runOnMainForNative<String?> {
      check(isInitialized) { "AngNative host not initialized" }
      nativeHostProcessBatch(hostHandle, batchJson)
    }
    return result ?: "{\"error\":\"null result\"}"
  }

  /**
   * Process a versioned ANB1 batch without copying it through a JNI string.
   * The direct buffer is allocated per call and borrowed only while the
   * synchronous native call runs.
   */
  fun processBatchBinary(batchBytes: ByteArray): String {
    val buffer = ByteBuffer.allocateDirect(batchBytes.size)
    buffer.put(batchBytes)
    buffer.flip()
    return processBatchDirect(buffer)
  }

  /**
   * Borrow a caller-owned direct buffer for one synchronous call. Instrumentation
   * tests reuse this entry point to prove Rust keeps no pointer after return.
   */
  fun processBatchDirect(buffer: ByteBuffer): String {
    val result = runOnMainForNative<String?> {
      check(isInitialized) { "AngNative host not initialized" }
      nativeHostProcessBatchBytes(hostHandle, buffer)
    }
    return result ?: "{\"error\":\"null result\"}"
  }

  /** Run layout on the current shadow tree. */
  fun runLayout(width: Float, height: Float): String {
    val result = runOnMainForNative<String?> {
      check(isInitialized) { "AngNative host not initialized" }
      nativeHostRunLayout(hostHandle, width, height)
    }
    return result ?: "{\"error\":\"layout failed\"}"
  }

  /** Get the last error from the Rust host. */
  fun lastError(): String {
    return runOnMainForNative<String?> { nativeHostLastError(hostHandle) } ?: "{\"ok\":true}"
  }

  /** Get the total node count in the shadow tree. */
  fun nodeCount(): Int = runOnMainForNative {
    if (!isInitialized) 0 else nativeHostNodeCount(hostHandle)
  }

  /** Take a snapshot of the current shadow tree as parsed objects. */
  fun snapshot(): SnapshotResult = runOnMainForNative {
    check(isInitialized) { "AngNative host not initialized" }
    val json = nativeHostSnapshot(hostHandle) ?: "{\"error\":\"snapshot failed\"}"
    SnapshotResult.parse(json)
  }

  fun destroyHost() = runOnMainForNative {
    // Drop queued native work before releasing the bridge so stale posts cannot
    // touch a released view factory or a destroyed host.
    mainHandler.removeCallbacksAndMessages(null)
    if (runtimeBridgeHandle != 0L) {
      // Pass hostHandle so C++ releases the ViewFactory JNI global ref and
      // g_contexts entry as well — otherwise it leaks on every recreation.
      nativeRuntimeBridgeDestroy(runtimeBridgeHandle, hostHandle)
      runtimeBridgeHandle = 0L
    }
    hostHandle = 0L
    viewFactory = null
    isInitialized = false
  }

  /* =======================================================================
   * Callbacks invoked from C++ (ViewFactory PlatformOps).
   * Each has a simple JNI-friendly signature; C++ resolves these via GetMethodID.
   * ======================================================================= */

  fun onNativeCreateView(nodeId: Long, viewType: String) {
    viewFactory?.onNativeCreateView(nodeId, viewType)
  }

  fun onNativeSetFrame(nodeId: Long, x: Float, y: Float, width: Float, height: Float) {
    viewFactory?.onNativeSetFrame(nodeId, x, y, width, height)
  }

  fun onNativeSetProps(nodeId: Long, propsJson: String) {
    viewFactory?.onNativeSetProps(nodeId, propsJson)
  }

  fun onNativeSetText(nodeId: Long, text: String) {
    viewFactory?.onNativeSetText(nodeId, text)
  }

  fun onNativeAppendChild(parentId: Long, childId: Long) {
    viewFactory?.onNativeAppendChild(parentId, childId)
  }

  fun onNativeInsertChild(parentId: Long, childId: Long, index: Int) {
    viewFactory?.onNativeInsertChild(parentId, childId, index)
  }

  fun onNativeRemoveChild(parentId: Long, childId: Long) {
    viewFactory?.onNativeRemoveChild(parentId, childId)
  }

  fun onNativeDestroyView(nodeId: Long) {
    viewFactory?.onNativeDestroyView(nodeId)
  }

  /**
   * Called by nativeHostRunLayout to build a LayoutRequestJson from the live
   * shadow-tree snapshot. Snapshot node shape (from host-core):
   *   { "nodes": [ { "id", "tag", "node_type", "parent_id", "children",
   *                  "styles", "properties", "text_content" } ] }
   * LayoutNodeInputJson shape: { "node_id", "styles", "children", "text_content?" }.
   */
  fun buildLayoutRequest(snapshotJson: String, width: Float, height: Float): String {
    val nodes = JSONArray()
    var rootId = 1L
    try {
      val root = JSONObject(snapshotJson)
      val arr = root.optJSONArray("nodes") ?: JSONArray()
      var foundRoot = false
      for (i in 0 until arr.length()) {
        val node = arr.getJSONObject(i)
        val id = node.optLong("id")
        if (!foundRoot && (node.isNull("parent_id") || node.optLong("parent_id", -1L) < 0L)) {
          rootId = id
          foundRoot = true
        }

        val styles = JSONObject()
        val stylesObj = node.optJSONObject("styles")
        if (stylesObj != null) {
          for (key in stylesObj.keys()) styles.put(key, stylesObj.optString(key))
        }

        val children = JSONArray()
        val childrenArr = node.optJSONArray("children")
        if (childrenArr != null) {
          for (j in 0 until childrenArr.length()) children.put(childrenArr.optLong(j))
        }

        val input = JSONObject()
        input.put("node_id", id)
        input.put("styles", styles)
        input.put("children", children)
        val nodeType = node.optString("node_type", "")
        val tag = node.optString("tag", "").lowercase()
        val isTextNode = nodeType == "Text" ||
          nodeType.startsWith("Custom:na-text") ||
          tag.startsWith("na-text") || tag.startsWith("native-text") ||
          tag == "text" || tag == "span" || tag == "p"
        if (isTextNode && !node.isNull("text_content")) {
          input.put("text_content", node.getString("text_content"))
        }
        nodes.put(input)
      }
    } catch (e: Exception) {
      Log.e(TAG, "buildLayoutRequest: failed to parse snapshot", e)
    }

    val request = JSONObject()
    request.put("root_id", rootId)
    request.put("nodes", nodes) // real nodes, never hardcoded empty
    request.put("available_width", width.toDouble())
    request.put("available_height", height.toDouble())
    return request.toString()
  }

  /**
   * Handle a native press event from a Rust-created view.
   *
   * The click originates in a native Button created by a Rust Create op; the
   * listener calls this method, which dispatches a native event through the
   * AngNative runtime bridge. This enables the Angular listener round trip:
   * native click → runtime bridge → JS runtime → na_onEvent → Angular listener.
   */
  fun onNativePress(nodeId: Long) {
    dispatchNativeEvent(nodeId, "press")
  }

  /** Dispatch any native event name (Angular templates use "click"). */
  fun dispatchNativeEvent(nodeId: Long, eventName: String): Int = runOnMainForNative {
    if (!isInitialized) return@runOnMainForNative 0
    val payload = """{"source":"native","nodeId":$nodeId,"event":"$eventName"}"""
    val invoked = nativeRuntimeBridgeDispatchEvent(runtimeBridgeHandle, nodeId, eventName, payload)
    Log.i(TAG, "dispatchNativeEvent($nodeId, $eventName) -> listeners invoked: $invoked")
    invoked
  }

  /* =======================================================================
   * View construction / styling helpers (kept as public utilities).
   * ======================================================================= */

  /** Create an android.view.View for a native node type. */
  fun createView(nodeType: String): View {
    return when (nodeType) {
      "View", "Custom", "ScrollView" -> FrameLayout(context)
      "Text" -> TextView(context).apply {
        gravity = Gravity.CENTER_VERTICAL
        setTextColor(Color.BLACK)
      }
      "TextInput" -> EditText(context).apply { setTextColor(Color.BLACK) }
      "Button" -> Button(context).apply {
        setTextColor(Color.WHITE)
        setBackgroundColor(Color.parseColor("#007AFF"))
      }
      "Image" -> ImageView(context).apply { scaleType = ImageView.ScaleType.FIT_CENTER }
      "Switch" -> Switch(context)
      "ActivityIndicator" -> ProgressBar(context).apply { isIndeterminate = true }
      else -> View(context)
    }
  }

  /** Apply style properties to an android.view.View. */
  fun applyStyle(view: View, key: String, value: String) {
    when (key) {
      "width" -> {
        view.layoutParams.width =
          parseDimension(value, context.resources.displayMetrics.widthPixels).toInt()
        view.requestLayout()
      }
      "height" -> {
        view.layoutParams.height =
          parseDimension(value, context.resources.displayMetrics.heightPixels).toInt()
        view.requestLayout()
      }
      "padding" -> {
        val p = parseDimension(value, context.resources.displayMetrics.widthPixels).toInt()
        view.setPadding(p, p, p, p)
      }
      "color" -> if (view is TextView) view.setTextColor(parseColor(value))
      "backgroundColor" -> view.setBackgroundColor(parseColor(value))
      "fontSize" -> if (view is TextView) {
        // Logical px map to sp directly; the old scaledDensity divide made text tiny.
        view.textSize = value.removeSuffix("px").toFloatOrNull() ?: 15f
      }
      "fontWeight" -> if (view is TextView) {
        val weight = when (value) {
          "bold", "700", "800", "900" -> android.graphics.Typeface.BOLD
          else -> android.graphics.Typeface.NORMAL
        }
        view.typeface = android.graphics.Typeface.defaultFromStyle(weight)
      }
      "opacity" -> view.alpha = value.toFloatOrNull() ?: 1.0f
      "visibility" -> view.visibility = if (value == "hidden") View.GONE else View.VISIBLE
    }
  }

  private fun parseDimension(value: String, maxPixels: Int): Float {
    return when {
      value.endsWith("px") -> value.dropLast(2).toFloatOrNull() ?: 0f
      value.endsWith("dp") ->
        (value.dropLast(2).toFloatOrNull() ?: 0f) * context.resources.displayMetrics.density
      value.endsWith("%") -> (value.dropLast(1).toFloatOrNull() ?: 0f) / 100f * maxPixels
      value == "auto" -> maxPixels.toFloat()
      else -> value.toFloatOrNull() ?: 0f
    }
  }

  private fun parseColor(value: String): Int {
    if (value.startsWith("#")) {
      return try {
        Color.parseColor(value)
      } catch (e: Exception) {
        Color.BLACK
      }
    }
    return when (value.lowercase()) {
      "black" -> Color.BLACK
      "white" -> Color.WHITE
      "red" -> Color.RED
      "green" -> Color.GREEN
      "blue" -> Color.BLUE
      "gray", "grey" -> Color.GRAY
      "transparent" -> Color.TRANSPARENT
      else -> Color.BLACK
    }
  }
}

/** Snapshot result from the Rust host. */
data class SnapshotResult(
  val nodes: List<NodeSnapshot> = emptyList()
) {
  companion object {
    fun parse(json: String): SnapshotResult {
      val nodes = mutableListOf<NodeSnapshot>()
      try {
        val root = JSONObject(json)
        val nodesArr = root.optJSONArray("nodes")
        if (nodesArr != null) {
          for (i in 0 until nodesArr.length()) {
            val nodeObj = nodesArr.getJSONObject(i)
            val id = nodeObj.optLong("id")
            val nodeType = nodeObj.optString("node_type", nodeObj.optString("tag", "View"))
            val parentId =
              if (nodeObj.isNull("parent_id")) null else nodeObj.optLong("parent_id")
            val textContent =
              if (nodeObj.isNull("text_content")) "" else nodeObj.optString("text_content", "")
            val styles = mutableMapOf<String, String>()
            val stylesObj = nodeObj.optJSONObject("styles")
            if (stylesObj != null) {
              for (key in stylesObj.keys()) styles[key] = stylesObj.optString(key)
            }
            nodes.add(NodeSnapshot(id, nodeType, 0f, 0f, 0f, 0f, parentId, textContent, styles))
          }
        }
      } catch (e: Exception) {
        // Parse error — return empty snapshot
      }
      return SnapshotResult(nodes)
    }
  }
}

/** A single node in the snapshot. */
data class NodeSnapshot(
  val id: Long,
  val nodeType: String,
  val x: Float,
  val y: Float,
  val width: Float,
  val height: Float,
  val parentId: Long?,
  val textContent: String,
  val styles: MutableMap<String, String> = mutableMapOf(),
  val properties: MutableMap<String, String> = mutableMapOf()
)

