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
 * View factory callbacks are implemented in Kotlin and create android.view.View
 * instances for na-* elements.
 */

package com.angnative.app

import android.content.Context
import android.graphics.Color
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.*
import androidx.core.widget.NestedScrollView

class JniBridge(private val context: Context) {

  // Native methods loaded from libangnative.so
  external fun nativeHostCreate(): Long
  external fun nativeHostDestroy(handle: Long)
  external fun nativeHostRegisterViewFactory(handle: Long, callbackPtr: Long)
  external fun nativeHostProcessBatch(handle: Long, batchJson: String): String?
  external fun nativeHostRunLayout(handle: Long, width: Float, height: Float): String?
  external fun nativeHostLastError(handle: Long): String?
  external fun nativeHostNodeCount(handle: Long): Int
  external fun nativeHostSnapshot(handle: Long): String?

  companion object {
    init {
      System.loadLibrary("angnative")
    }
  }

  private var hostHandle: Long = 0
  private var isInitialized: Boolean = false

  fun initHost() {
    if (isInitialized) return
    hostHandle = nativeHostCreate()
    if (hostHandle == 0L) {
      val err = nativeHostLastError(hostHandle)
      throw RuntimeException("Failed to create AngNative host: $err")
    }

    // Register view factory callbacks
    registerViewFactory(hostHandle)
    isInitialized = true
  }

  private fun registerViewFactory(handle: Long) {
    val callbackPtr = ViewFactoryCallbackPtr(context)
    nativeHostRegisterViewFactory(handle, callbackPtr.ptr)
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
    val json = nativeHostSnapshot(hostHandle) ?: "{\"nodes\":[]}"
    return SnapshotResult.parse(json)
  }

  fun destroyHost() {
    if (hostHandle != 0L) {
      nativeHostDestroy(hostHandle)
      hostHandle = 0L
    }
    isInitialized = false
  }

  /** Create an android.view.View for a native node type. */
  fun createView(nodeType: String): View {
    return when (nodeType) {
      "View", "Custom", "ScrollView" -> {
        val vg = FrameLayout(context)
        vg.layoutParams = ViewGroup.LayoutParams(
          ViewGroup.LayoutParams.MATCH_PARENT,
          ViewGroup.LayoutParams.WRAP_CONTENT
        )
        vg
      }
      "Text" -> {
        TextView(context).apply {
          gravity = Gravity.CENTER_VERTICAL
          setTextColor(Color.BLACK)
        }
      }
      "TextInput" -> {
        EditText(context).apply {
          setTextColor(Color.BLACK)
          setHintTextColor(Color.GRAY)
        }
      }
      "Button" -> {
        Button(context).apply {
          setTextColor(Color.WHITE)
          setBackgroundColor(Color.parseColor("#007AFF"))
        }
      }
      "Image" -> {
        ImageView(context).apply {
          scaleType = ImageView.ScaleType.FIT_CENTER
        }
      }
      "Switch" -> {
        Switch(context)
      }
      "ActivityIndicator" -> {
        ProgressBar(context).apply {
          isIndeterminate = true
        }
      }
      else -> View(context)
    }
  }

  /** Apply style properties to an android.view.View. */
  fun applyStyle(view: View, key: String, value: String) {
    when (key) {
      "width" -> {
        val w = parseDimension(value, context.resources.displayMetrics.widthPixels)
        view.layoutParams.width = w.toInt()
        view.requestLayout()
      }
      "height" -> {
        val h = parseDimension(value, context.resources.displayMetrics.heightPixels)
        view.layoutParams.height = h.toInt()
        view.requestLayout()
      }
      "padding" -> {
        val p = parseDimension(value, context.resources.displayMetrics.widthPixels).toInt()
        view.setPadding(p, p, p, p)
      }
      "paddingTop" -> {
        val p = parseDimension(value, context.resources.displayMetrics.widthPixels).toInt()
        view.setPadding(view.paddingLeft, p, view.paddingRight, view.paddingBottom)
      }
      "paddingBottom" -> {
        val p = parseDimension(value, context.resources.displayMetrics.widthPixels).toInt()
        view.setPadding(view.paddingLeft, view.paddingTop, view.paddingRight, p)
      }
      "paddingLeft" -> {
        val p = parseDimension(value, context.resources.displayMetrics.widthPixels).toInt()
        view.setPadding(p, view.paddingTop, view.paddingRight, view.paddingBottom)
      }
      "paddingRight" -> {
        val p = parseDimension(value, context.resources.displayMetrics.widthPixels).toInt()
        view.setPadding(view.paddingLeft, view.paddingTop, p, view.paddingBottom)
      }
      "color" -> {
        if (view is TextView) {
          view.setTextColor(parseColor(value))
        }
      }
      "backgroundColor" -> {
        view.setBackgroundColor(parseColor(value))
      }
      "fontSize" -> {
        if (view is TextView) {
          val sp = parseDimension(value, context.resources.displayMetrics.widthPixels) /
            context.resources.displayMetrics.scaledDensity
          view.textSize = sp
        }
      }
      "fontWeight" -> {
        if (view is TextView) {
          val weight = when (value) {
            "bold", "700", "800", "900" -> android.graphics.Typeface.BOLD
            else -> android.graphics.Typeface.NORMAL
          }
          view.typeface = android.graphics.Typeface.defaultFromStyle(weight)
        }
      }
      "opacity" -> {
        view.alpha = value.toFloatOrNull() ?: 1.0f
      }
      "visibility" -> {
        view.visibility = when (value) {
          "hidden" -> View.GONE
          else -> View.VISIBLE
        }
      }
    }
  }

  private fun parseDimension(value: String, maxPixels: Int): Float {
    return when {
      value.endsWith("px") -> value.dropLast(2).toFloatOrNull() ?: 0f
      value.endsWith("dp") -> (value.dropLast(2).toFloatOrNull() ?: 0f) * context.resources.displayMetrics.density
      value.endsWith("%") -> {
        val pct = (value.dropLast(1).toFloatOrNull() ?: 0f) / 100f
        maxPixels * pct
      }
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

  /** Handle a native press event from the Rust host. */
  fun onNativePress(nodeId: Long) {
    // Find the view and trigger click
  }
}

/** Simple wrapper for view factory callback pointer. */
class ViewFactoryCallbackPtr(private val context: Context) {
  val ptr: Long = System.identityHashCode(this).toLong()
}

/** Snapshot result from the Rust host. */
data class SnapshotResult(
  val nodes: List<NodeSnapshot> = emptyList()
) {
  companion object {
    fun parse(json: String): SnapshotResult {
      val nodes = mutableListOf<NodeSnapshot>()
      try {
        val root = org.json.JSONObject(json)
        val nodesArr = root.optJSONArray("nodes")
        if (nodesArr != null) {
          for (i in 0 until nodesArr.length()) {
            val nodeObj = nodesArr.getJSONObject(i)
            val id = nodeObj.optLong("id")
            val nodeType = nodeObj.optString("node_type", nodeObj.optString("tag", "View"))
            val parentId = if (nodeObj.isNull("parent_id")) null else nodeObj.optLong("parent_id")
            val textContent = if (nodeObj.isNull("text_content")) "" else nodeObj.optString("text_content", "")
            val styles = mutableMapOf<String, String>()
            val stylesObj = nodeObj.optJSONObject("styles")
            if (stylesObj != null) {
              for (key in stylesObj.keys()) {
                styles[key] = stylesObj.optString(key)
              }
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
