/**
 * NativeViewFactory.kt — reusable materialization of Rust-emitted PlatformOps
 * into real Android views.
 *
 * Extracted from the eight showcase copies so there is one implementation of
 * the node registry, geometry, styling and tree mutation logic. The node
 * registry (nodeId -> View) is the single source of truth: no UI content is
 * built here beyond what the Rust host emits through the eight callbacks.
 *
 * The public `onNative*` method names/signatures are unchanged from the
 * showcase `com.angnative.app.NativeViewFactory`; a host JniBridge can keep
 * resolving/calling them exactly as before and simply delegate.
 *
 * NOTE: callbacks must be delivered on the main/UI thread (the C++ bridge
 * should dispatch there), as with the original implementation.
 */
package com.angnative.ui.core

import android.content.Context
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

/**
 * Optional override for view construction. Return null to request the stock
 * view for that node type, which lets a showcase customize only selected
 * widgets, e.g.
 *
 * ```
 * viewCreator = { id, type ->
 *   if (type == "Button") myBrandedButton(id) else defaultViewForType(id, type)
 * }
 * ```
 */
typealias NativeViewCreator = (nodeId: Long, viewType: String) -> View?

class NativeViewFactory(
  private val context: Context,
  private val rootContainer: ViewGroup,
  private val onPress: (Long) -> Unit = {},
  private val theme: NativeUiTheme = NativeUiTheme.DEFAULT,
  private val viewCreator: NativeViewCreator? = null
) {
  private val views = LinkedHashMap<Long, View>()
  private val rootIds = HashSet<Long>()
  private val density: Float = context.resources.displayMetrics.density
  private val widthPixels: Int = context.resources.displayMetrics.widthPixels
  private val fontScale: Float = context.resources.configuration.fontScale

  /** Number of live nodes in the registry. */
  val viewCount: Int get() = views.size

  /** Look up a live view (useful for diagnostics and integration tests). */
  fun getView(nodeId: Long): View? = views[nodeId]

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

  /** PlatformOp::Create — JNI: (JLjava/lang/String;)V */
  fun onNativeCreateView(nodeId: Long, viewType: String) {
    if (views.containsKey(nodeId)) return
    val view = createViewForType(nodeId, viewType)
    view.layoutParams = frameParams()
    views[nodeId] = view
    if (nodeId in rootIds && view.parent == null) {
      rootContainer.addView(view, frameParams())
    }
  }

  /** PlatformOp::SetFrame — geometry from LayoutResponseJson (dp -> px). JNI: (JFFFF)V */
  fun onNativeSetFrame(nodeId: Long, x: Float, y: Float, width: Float, height: Float) {
    val view = views[nodeId] ?: return
    val lp = view.layoutParams as? FrameLayout.LayoutParams ?: frameParams()
    // Keep WRAP_CONTENT when layout reports no usable dimension (e.g. text).
    if (width > 0f) lp.width = (width * density).toInt()
    if (height > 0f) lp.height = (height * density).toInt()
    lp.leftMargin = (x * density).toInt()
    lp.topMargin = (y * density).toInt()
    view.layoutParams = lp
  }

  /**
   * PlatformOp::SetProps — ViewProps JSON: { text, styles, properties }.
   * JNI: (JLjava/lang/String;)V
   */
  fun onNativeSetProps(nodeId: Long, propsJson: String) {
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

  /** PlatformOp::SetText — JNI: (JLjava/lang/String;)V */
  fun onNativeSetText(nodeId: Long, text: String) {
    val view = views[nodeId] ?: return
    if (view is TextView) view.text = text
  }

  /** PlatformOp::Append — JNI: (JJ)V */
  fun onNativeAppendChild(parentId: Long, childId: Long) {
    attachChild(parentId, childId, -1)
  }

  /** PlatformOp::Insert — JNI: (JJI)V */
  fun onNativeInsertChild(parentId: Long, childId: Long, index: Int) {
    attachChild(parentId, childId, index)
  }

  /** PlatformOp::Remove — JNI: (JJ)V */
  fun onNativeRemoveChild(parentId: Long, childId: Long) {
    val parent = views[parentId] as? ViewGroup ?: return
    val child = views[childId] ?: return
    parent.removeView(child)
  }

  /** PlatformOp::Destroy — JNI: (J)V */
  fun onNativeDestroyView(nodeId: Long) {
    val view = views.remove(nodeId) ?: return
    (view.parent as? ViewGroup)?.removeView(view)
    rootIds.remove(nodeId)
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

  /** Drop all nodes and detach every root from the container. */
  fun clear() {
    views.clear()
    rootIds.clear()
    rootContainer.removeAllViews()
  }

  /**
   * Apply one style key/value to a view. Public so custom [viewCreator]
   * implementations can reuse the stock styling behavior.
   */
  fun applyStyle(view: View, key: String, value: String) {
    when (key) {
      "width" -> {
        view.layoutParams.width = parsePixels(value)
        view.requestLayout()
      }
      "height" -> {
        view.layoutParams.height = parsePixels(value)
        view.requestLayout()
      }
      "padding" -> {
        val p = parsePadding(value)
        view.setPadding(p, p, p, p)
      }
      "paddingTop", "paddingBottom", "paddingLeft", "paddingRight" -> {
        val p = parsePadding(value)
        view.setPadding(
          if (key == "paddingLeft") p else view.paddingLeft,
          if (key == "paddingTop") p else view.paddingTop,
          if (key == "paddingRight") p else view.paddingRight,
          if (key == "paddingBottom") p else view.paddingBottom
        )
      }
      "color" -> if (view is TextView) view.setTextColor(StyleCodec.parseColor(value))
      "backgroundColor" -> view.setBackgroundColor(StyleCodec.parseColor(value))
      "fontSize" -> if (view is TextView) {
        view.textSize = StyleCodec.parseTextSize(value, density, fontScale, theme.textSizeSp)
      }
      "fontWeight" -> if (view is TextView) {
        view.typeface = Typeface.defaultFromStyle(StyleCodec.parseFontWeight(value))
      }
      "opacity" -> view.alpha = StyleCodec.parseOpacity(value)
      "visibility" -> view.visibility = StyleCodec.parseVisibility(value)
      // flexDirection/margin/etc. are layout-engine concerns; ignored here.
    }
  }

  /** Build the stock view for a node type. Custom creators may delegate here. */
  fun defaultViewForType(nodeId: Long, viewType: String): View = when (ViewTypes.kindFor(viewType)) {
    NativeViewKind.CONTAINER -> FrameLayout(context)
    NativeViewKind.TEXT -> TextView(context).apply {
      gravity = Gravity.CENTER_VERTICAL
      setTextColor(theme.textColor)
      textSize = theme.textSizeSp
    }
    NativeViewKind.TEXT_INPUT -> EditText(context).apply { setTextColor(theme.inputTextColor) }
    NativeViewKind.BUTTON -> Button(context).apply {
      isAllCaps = theme.buttonAllCaps
      setTextColor(theme.buttonTextColor)
      textSize = theme.buttonTextSizeSp
      typeface = Typeface.DEFAULT_BOLD
      background = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = theme.buttonCornerRadiusDp * density
        setColor(theme.buttonBackground)
      }
      setOnClickListener { onPress(nodeId) }
    }
    NativeViewKind.IMAGE -> ImageView(context).apply { scaleType = ImageView.ScaleType.FIT_CENTER }
    NativeViewKind.SWITCH -> Switch(context)
    NativeViewKind.ACTIVITY_INDICATOR -> ProgressBar(context).apply { isIndeterminate = true }
    NativeViewKind.PLAIN -> View(context)
  }

  private fun createViewForType(nodeId: Long, viewType: String): View =
    viewCreator?.invoke(nodeId, viewType) ?: defaultViewForType(nodeId, viewType)

  private fun attachChild(parentId: Long, childId: Long, index: Int) {
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

  private fun parsePixels(value: String): Int =
    StyleCodec.parseDimension(value).toPixels(density, widthPixels)

  /** Padding has no meaningful WRAP_CONTENT value; auto maps to zero. */
  private fun parsePadding(value: String): Int {
    val dim = StyleCodec.parseDimension(value)
    return if (dim.unit == StyleCodec.DimensionUnit.AUTO) 0 else dim.toPixels(density, widthPixels)
  }
}
