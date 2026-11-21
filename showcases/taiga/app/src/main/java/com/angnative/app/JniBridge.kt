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
 * methods below, which delegate to the installed NativeViewFactory.
 */

package com.angnative.app

import android.content.Context
import android.graphics.Color
import android.util.Log
import android.view.Gravity
import android.view.View
import android.widget.*
import org.json.JSONArray
import org.json.JSONObject

class JniBridge(private val context: Context) {

  companion object {
    private const val TAG = "AngNative"
    /** Node bound to the demo's output Text, updated by native presses. */
    private const val OUTPUT_NODE_ID = 6L
    /** Node bound to the demo's Reset button. */
    private const val RESET_NODE_ID = 7L

    init {
      System.loadLibrary("angnative")
    }
  }

  // Native methods loaded from libangnative.so
  external fun nativeHostCreate(): Long
  external fun nativeHostDestroy(handle: Long)
  external fun nativeHostRegisterViewFactory(handle: Long)
  external fun nativeHostProcessBatch(handle: Long, batchJson: String): String?
  external fun nativeHostRunLayout(handle: Long, width: Float, height: Float): String?
  external fun nativeHostLastError(handle: Long): String?
  external fun nativeHostNodeCount(handle: Long): Int
  external fun nativeHostSnapshot(handle: Long): String?

  private var hostHandle: Long = 0
  private var isInitialized: Boolean = false
  private var pressCount: Int = 0

  /** Installed by the platform view; receives the Rust ViewFactory callbacks. */
  var viewFactory: NativeViewFactory? = null

  fun initHost() {
    if (isInitialized) return
    hostHandle = nativeHostCreate()
    if (hostHandle == 0L) {
      val err = nativeHostLastError(hostHandle)
      throw RuntimeException("Failed to create AngNative host: $err")
    }

    // C++ allocates a real ViewFactoryContext (JNI global ref) from `this`.
    nativeHostRegisterViewFactory(hostHandle)
    isInitialized = true
  }

  fun initHostWithCallbacks() {
    initHost()
  }

  /** Process a JSON command batch through the Rust host. */
  fun processBatch(batchJson: String): String {
    check(isInitialized) { "AngNative host not initialized" }
    return nativeHostProcessBatch(hostHandle, batchJson) ?: "{\"error\":\"null result\"}"
  }

  /** Run layout on the current shadow tree. */
  fun runLayout(width: Float, height: Float): String {
    check(isInitialized) { "AngNative host not initialized" }
    return nativeHostRunLayout(hostHandle, width, height) ?: "{\"error\":\"layout failed\"}"
  }

  /** Get the last error from the Rust host. */
  fun lastError(): String {
    return nativeHostLastError(hostHandle) ?: "{\"ok\":true}"
  }

  /** Get the total node count in the shadow tree. */
  fun nodeCount(): Int {
    if (!isInitialized) return 0
    return nativeHostNodeCount(hostHandle)
  }

  /** Take a snapshot of the current shadow tree as parsed objects. */
  fun snapshot(): SnapshotResult {
    check(isInitialized) { "AngNative host not initialized" }
    val json = nativeHostSnapshot(hostHandle) ?: "{\"error\":\"snapshot failed\"}"
    return SnapshotResult.parse(json)
  }

  fun destroyHost() {
    if (hostHandle != 0L) {
      nativeHostDestroy(hostHandle) // C++ deletes the context + JNI global ref
      hostHandle = 0L
    }
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
   * LayoutNodeInputJson shape: { "node_id", "styles", "children" }.
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
   * listener calls this method, which dispatches an imperative command batch to
   * the Rust host (SetTextContent on the bound output node). A full Angular
   * listener round trip requires the embedded JS runtime, which is not wired
   * up on Android yet — this path does not claim to reach Angular.
   */
  fun onNativePress(nodeId: Long) {
    if (!isInitialized) return
    if (nodeId == RESET_NODE_ID) {
      pressCount = 0
      processBatch(buildBatch { setText(OUTPUT_NODE_ID, "") })
    } else {
      pressCount++
      val value = if (pressCount == 1) "Hello World!" else "Hello World! (x$pressCount)"
      val response = processBatch(buildBatch { setText(OUTPUT_NODE_ID, value) })
      Log.i(TAG, "onNativePress($nodeId) dispatched to Rust host: $response")
    }
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

