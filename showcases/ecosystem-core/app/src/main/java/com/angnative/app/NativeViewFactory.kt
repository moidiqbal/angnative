/**
 * NativeViewFactory.kt — Materializes Rust-emitted PlatformOps into real
 * Android views.
 *
 * The node registry (nodeId -> View) is the single source of truth. Views are
 * created, attached, framed, styled and given text ONLY through callbacks
 * invoked by angnative-host-ffi (via JniBridge); no UI content is built here
 * beyond what the Rust host emits.
 */

package com.angnative.app

import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.ProgressBar
import android.widget.Switch
import android.widget.TextView
import org.json.JSONObject

class NativeViewFactory(
  private val context: Context,
  private val rootContainer: ViewGroup,
  private val onPress: (Long) -> Unit = {}
) : NativeViewSink {
  private val views = mutableMapOf<Long, View>()
  private val rootIds = mutableSetOf<Long>()
  private val childParents = mutableMapOf<Long, Long>()
  private val density = context.resources.displayMetrics.density

  /** Mark a node as a root: it will be attached directly to [rootContainer]. */
  fun setRootNode(nodeId: Long) {
    rootIds.add(nodeId)
  }

  /** Attach a created root node to the container (called after the batch commit). */
  fun attachRoot(nodeId: Long) {
    val view = views[nodeId] ?: return
    if (view.parent == null) {
      rootContainer.addView(view, frameParams())
    }
  }

  /** PlatformOp::Create */
  override fun onNativeCreateView(nodeId: Long, viewType: String) {
    if (views.containsKey(nodeId)) return
    val view = createViewForType(nodeId, viewType)
    view.layoutParams = frameParams()
    views[nodeId] = view
    // Angular registers (click) listeners on component host nodes, which may
    // be ancestors of the tappable text view. Make every node clickable so a
    // real tap can be dispatched up the ancestor chain.
    view.isClickable = true
    view.setOnClickListener { onPress(nodeId) }
    if (nodeId in rootIds && view.parent == null) {
      rootContainer.addView(view, frameParams())
    }
  }

  /** PlatformOp::SetFrame — geometry from LayoutResponseJson (dp -> px). */
  override fun onNativeSetFrame(nodeId: Long, x: Float, y: Float, width: Float, height: Float) {
    val view = views[nodeId] ?: return
    val lp = view.layoutParams as? FrameLayout.LayoutParams ?: frameParams()
    // Keep WRAP_CONTENT when layout reports no usable dimension (e.g. text).
    if (width > 0f) lp.width = (width * density).toInt()
    if (height > 0f) lp.height = (height * density).toInt()
    lp.leftMargin = (x * density).toInt()
    lp.topMargin = (y * density).toInt()
    view.layoutParams = lp
  }

  /** PlatformOp::SetProps — ViewProps JSON { text, styles, properties }. */
  override fun onNativeSetProps(nodeId: Long, propsJson: String) {
    val view = views[nodeId] ?: return
    try {
      val props = JSONObject(propsJson)
      val styles = props.optJSONObject("styles")
      if (styles != null) {
        for (key in styles.keys()) applyStyle(view, key, styles.optString(key))
      }
      val properties = props.optJSONObject("properties")
      if (properties != null) {
        for (key in properties.keys()) applyStyle(view, key, properties.optString(key))
      }
      // `optString` returns the literal "null" for JSON null. Only a real
      // string value may overwrite text set by a SetText op.
      if (!props.isNull("text") && view is TextView) {
        val text = props.optString("text", "")
        if (text.isNotEmpty()) view.text = text
      }
    } catch (e: Exception) {
      // Malformed props payload — leave the view untouched.
    }
  }

  /** PlatformOp::SetText */
  override fun onNativeSetText(nodeId: Long, text: String) {
    val view = views[nodeId] ?: return
    if (view is TextView) view.text = text
  }

  /** PlatformOp::Append */
  override fun onNativeAppendChild(parentId: Long, childId: Long) {
    attachChild(parentId, childId, -1)
  }

  /** PlatformOp::Insert */
  override fun onNativeInsertChild(parentId: Long, childId: Long, index: Int) {
    attachChild(parentId, childId, index)
  }

  /** PlatformOp::Remove */
  override fun onNativeRemoveChild(parentId: Long, childId: Long) {
    val parent = views[parentId] as? ViewGroup ?: return
    val child = views[childId] ?: return
    parent.removeView(child)
  }

  /** PlatformOp::Destroy */
  override fun onNativeDestroyView(nodeId: Long) {
    val view = views.remove(nodeId) ?: return
    (view.parent as? ViewGroup)?.removeView(view)
  }

  /** Apply a LayoutResponseJson to the registry: { "nodes": [rects] }. */
  fun applyLayoutResult(layoutJson: String) {
    try {
      val response = JSONObject(layoutJson)
      val rects = response.optJSONArray("nodes") ?: return
      for (i in 0 until rects.length()) {
        val rect = rects.getJSONObject(i)
        onNativeSetFrame(
          rect.optLong("node_id"),
          rect.optDouble("x").toFloat(),
          rect.optDouble("y").toFloat(),
          rect.optDouble("width").toFloat(),
          rect.optDouble("height").toFloat()
        )
      }
    } catch (e: Exception) {
      // Layout error surfaced in the host's last_error; nothing to apply.
    }
  }

  /** Node id of the first Button whose text matches (used by E2E tap proofs). */
  fun findButtonIdByText(text: String): Long? =
    views.entries.firstOrNull { (_, v) -> v is TextView && v.text.toString() == text }?.key

  /** Current text of a native TextView node (used by E2E assertions). */
  fun textOf(nodeId: Long): String? = (views[nodeId] as? TextView)?.text?.toString()

  fun clear() {
    views.clear()
    rootIds.clear()
    childParents.clear()
    rootContainer.removeAllViews()
  }

  fun parentOf(nodeId: Long): Long? = childParents[nodeId]

  private fun attachChild(parentId: Long, childId: Long, index: Int) {
    childParents[childId] = parentId
    val parent = views[parentId] as? ViewGroup ?: return
    val child = views[childId] ?: return
    (child.parent as? ViewGroup)?.removeView(child)
    val lp = frameParams()
    if (index in 0..parent.childCount) {
      parent.addView(child, index, lp)
    } else {
      parent.addView(child, lp)
    }
  }

  private fun frameParams(): FrameLayout.LayoutParams =
    FrameLayout.LayoutParams(
      FrameLayout.LayoutParams.WRAP_CONTENT,
      FrameLayout.LayoutParams.WRAP_CONTENT
    )

  private fun createViewForType(nodeId: Long, viewType: String): View {
    return when (viewType) {
      "View", "Custom", "ScrollView" -> FrameLayout(context)
      "Text" -> TextView(context).apply {
        gravity = Gravity.CENTER_VERTICAL
        setTextColor(Color.parseColor("#0F172A"))
        textSize = 15f
      }
      "TextInput" -> EditText(context).apply { setTextColor(Color.BLACK) }
      "Button" -> Button(context).apply {
        isAllCaps = false
        setTextColor(Color.WHITE)
        textSize = 16f
        typeface = Typeface.DEFAULT_BOLD
        background = GradientDrawable().apply {
          shape = GradientDrawable.RECTANGLE
          cornerRadius = 12f * density
          setColor(Color.parseColor("#2563EB"))
        }
        setOnClickListener { onPress(nodeId) }
      }
      "Image" -> ImageView(context).apply { scaleType = ImageView.ScaleType.FIT_CENTER }
      "Switch" -> Switch(context)
      "ActivityIndicator" -> ProgressBar(context).apply { isIndeterminate = true }
      else -> View(context)
    }
  }

  private fun applyStyle(view: View, key: String, value: String) {
    when (key) {
      "width" -> {
        view.layoutParams.width = parseDimension(value).toInt()
        view.requestLayout()
      }
      "height" -> {
        view.layoutParams.height = parseDimension(value).toInt()
        view.requestLayout()
      }
      "padding" -> {
        val p = parseDimension(value).toInt()
        view.setPadding(p, p, p, p)
      }
      "paddingTop", "paddingBottom", "paddingLeft", "paddingRight" -> {
        val p = parseDimension(value).toInt()
        view.setPadding(
          if (key == "paddingLeft") p else view.paddingLeft,
          if (key == "paddingTop") p else view.paddingTop,
          if (key == "paddingRight") p else view.paddingRight,
          if (key == "paddingBottom") p else view.paddingBottom
        )
      }
      "color" -> if (view is TextView) view.setTextColor(parseColor(value))
      "backgroundColor" -> view.setBackgroundColor(parseColor(value))
      "fontSize" -> if (view is TextView) {
        view.textSize = parseTextSize(value)
      }
      "fontWeight" -> if (view is TextView) {
        val style = when (value) {
          "bold", "700", "800", "900" -> Typeface.BOLD
          else -> Typeface.NORMAL
        }
        view.typeface = Typeface.defaultFromStyle(style)
      }
      "opacity" -> view.alpha = value.toFloatOrNull() ?: 1f
      "visibility" -> view.visibility = if (value == "hidden") View.GONE else View.VISIBLE
      // flexDirection/margin/etc. are layout-engine concerns; ignored here.
    }
  }

  private fun parseDimension(value: String): Float {
    return when {
      value.endsWith("px") -> value.dropLast(2).toFloatOrNull() ?: 0f
      value.endsWith("dp") -> (value.dropLast(2).toFloatOrNull() ?: 0f) * density
      value.endsWith("%") ->
        (value.dropLast(1).toFloatOrNull() ?: 0f) / 100f * context.resources.displayMetrics.widthPixels
      value == "auto" -> ViewGroup.LayoutParams.WRAP_CONTENT.toFloat()
      else -> value.toFloatOrNull() ?: 0f
    }
  }

  /**
   * Font sizes from the mutation batch are logical/spec pixels, which map to
   * Android sp directly. Dividing by scaledDensity made 11px ≈ 4sp (tiny text).
   */
  private fun parseTextSize(value: String): Float {
    return when {
      value.endsWith("px") -> value.dropLast(2).toFloatOrNull() ?: 15f
      value.endsWith("dp") ->
        (value.dropLast(2).toFloatOrNull() ?: 15f) * density /
          context.resources.configuration.fontScale
      else -> value.toFloatOrNull() ?: 15f
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
