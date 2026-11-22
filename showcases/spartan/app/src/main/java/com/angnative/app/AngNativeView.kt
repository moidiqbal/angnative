/**
 * AngNativeView.kt ΓÇö Native container for AngNative on Android.
 *
 * The demo still dispatches a command batch to the Rust host, but the visible
 * UI is built exclusively from the PlatformOps Rust emits through the
 * registered ViewFactory (nodeId -> View registry, hierarchy, frames, props,
 * text). Kotlin only provides a thin decorative card around that content.
 */

package com.angnative.app

import android.content.Context
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.util.AttributeSet
import android.util.Log
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.LinearLayout
import androidx.core.widget.NestedScrollView

fun buildBatch(block: BatchBuilder.() -> Unit): String = BatchBuilder().apply(block).build()

class AngNativeView(context: Context, attrs: AttributeSet? = null) :
  FrameLayout(context, attrs) {

  private val tag = "AngNative"
  private lateinit var jniBridge: JniBridge
  private var viewFactory: NativeViewFactory? = null

  companion object {
    private const val ROOT_NODE_ID = 1L
  }

  init {
    jniBridge = JniBridge(context)
    jniBridge.initHost()

    layoutParams = LayoutParams(
      LayoutParams.MATCH_PARENT,
      LayoutParams.MATCH_PARENT
    )
    setBackgroundColor(Color.parseColor("#F8FAFC"))
  }

  /**
   * Loads the "Hello World" interactive demo.
   *
   * Dispatches a command batch to the Rust host; Rust then emits Create /
   * Insert / SetProps / SetText ops that materialize the real native views.
   */
  fun loadHelloWorldApp() {
    removeAllViews()

    val density = resources.displayMetrics.density
    fun toPx(dp: Int): Int = (dp * density).toInt()

    // Thin decorative shell. All visible content (nodes, text, hierarchy)
    // comes from the Rust ops applied by NativeViewFactory below.
    val scrollView = NestedScrollView(context).apply {
      layoutParams = LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT)
      isFillViewport = true
    }
    val shell = LinearLayout(context).apply {
      orientation = LinearLayout.VERTICAL
      layoutParams = LinearLayout.LayoutParams(
        LinearLayout.LayoutParams.MATCH_PARENT,
        LinearLayout.LayoutParams.WRAP_CONTENT
      )
      setPadding(toPx(16), toPx(16), toPx(16), toPx(24))
    }
    val card = FrameLayout(context).apply {
      layoutParams = LinearLayout.LayoutParams(
        LinearLayout.LayoutParams.MATCH_PARENT,
        LinearLayout.LayoutParams.WRAP_CONTENT
      )
      background = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = 18f * density
        setColor(Color.WHITE)
        setStroke(toPx(1), Color.parseColor("#E2E8F0"))
      }
      elevation = 4f * density
      setPadding(toPx(20), toPx(22), toPx(20), toPx(22))
    }
    shell.addView(card)
    scrollView.addView(shell)
    addView(scrollView)

    // Route Rust-emitted PlatformOps into real Android views.
    val factory = NativeViewFactory(
      context,
      card,
      onPress = { nodeId -> jniBridge.onNativePress(nodeId) }
    )
    factory.setRootNode(ROOT_NODE_ID)
    viewFactory = factory
    jniBridge.viewFactory = factory

    // Demo command batch ΓÇö the content of the UI lives here, not in Kotlin View code.
    // Explicit heights are required because intrinsic text measurement is still a
    // stub in the Rust layout engine (it reports 0 for text nodes).
    val initialBatch = buildBatch {
      // Root container (id=1)
      createNode(1, "View")
      setStyle(1, "flexDirection", "column")
      setStyle(1, "padding", "16px")
      // 82% leaves symmetric breathing room inside the card; 90/100% pushed
      // every child flush against the card's right edge (looked clipped).
      setStyle(1, "width", "82%")

      // Badge (id=2)
      createNode(2, "Text")
      setText(2, "ANGNATIVE ΓÇó SPARTAN UI SHOWCASE (NATIVE REIMPLEMENTATION)")
      setStyle(2, "fontSize", "11px")
      setStyle(2, "fontWeight", "bold")
      setStyle(2, "color", "#4338CA")
      setStyle(2, "height", "30px")
      insertChild(1, 2, 0)

      // Title (id=3)
      createNode(3, "Text")
      setText(3, "Spartan UI ΓÇö Native Showcase")
      setStyle(3, "fontSize", "26px")
      setStyle(3, "fontWeight", "bold")
      setStyle(3, "color", "#0F172A")
      setStyle(3, "height", "72px")
      insertChild(1, 3, 1)

      // Subtitle (id=4)
      createNode(4, "Text")
      setText(4, "Tap the button below to dispatch a native event through the AngNative Rust host.")
      setStyle(4, "fontSize", "14px")
      setStyle(4, "color", "#64748B")
      setStyle(4, "height", "64px")
      insertChild(1, 4, 2)

      // Button (id=5)
      createNode(5, "Button")
      setText(5, "Click Me!")
      setStyle(5, "height", "50px")
      insertChild(1, 5, 3)

      // Output text (id=6)
      createNode(6, "Text")
      setText(6, "")
      setStyle(6, "fontSize", "18px")
      setStyle(6, "color", "#1D4ED8")
      setStyle(6, "height", "40px")
      insertChild(1, 6, 4)

      // Reset button (id=7)
      createNode(7, "Button")
      setText(7, "Reset")
      setStyle(7, "height", "44px")
      insertChild(1, 7, 5)

      // Footer (id=8) ΓÇö neutral wording, no stale layout-engine claim.
      createNode(8, "Text")
      setText(8, "AngNative Runtime v0.1.0 ΓÇó Rust FFI ΓÇó Android shell")
      setStyle(8, "fontSize", "12px")
      setStyle(8, "color", "#94A3B8")
      setStyle(8, "height", "30px")
      insertChild(1, 8, 6)
    }

    val initResponse = jniBridge.processBatch(initialBatch)
    Log.i(tag, "Initial batch dispatched to Rust host. Ops: $initResponse")
    Log.i(tag, "Total nodes in Rust shadow tree: ${jniBridge.nodeCount()}")

    factory.attachRoot(ROOT_NODE_ID)

    // Run layout on the real tree: nativeHostRunLayout builds its request from
    // angnative_host_snapshot(), then the returned rects are applied as frames.
    val widthDp = resources.displayMetrics.widthPixels / density
    val heightDp = resources.displayMetrics.heightPixels / density
    val layoutJson = jniBridge.runLayout(widthDp, heightDp)
    factory.applyLayoutResult(layoutJson)
    Log.i(tag, "Layout response: $layoutJson")
  }

  /** Alias to load the demo app. */
  fun loadDemoApp() {
    loadHelloWorldApp()
  }

  fun onResume() {}
  fun onPause() {}

  fun destroy() {
    jniBridge.destroyHost()
    viewFactory?.clear()
    viewFactory = null
    removeAllViews()
  }
}

/** Helper to build a command batch as JSON. */
class BatchBuilder {
  private val commands: MutableList<String> = mutableListOf()

  fun createNode(id: Long, nodeType: String) {
    commands.add("""{"kind":"CreateNode","id":$id,"nodeType":"$nodeType"}""")
  }

  fun setStyle(id: Long, key: String, value: String) {
    commands.add("""{"kind":"SetStyle","id":$id,"key":"$key","value":"$value"}""")
  }

  fun setText(id: Long, text: String) {
    commands.add("""{"kind":"SetTextContent","id":$id,"text":"$text"}""")
  }

  fun insertChild(parentId: Long, childId: Long, index: Int) {
    commands.add("""{"kind":"InsertChild","parentId":$parentId,"childId":$childId,"index":$index}""")
  }

  fun build(): String {
    return """{"protocolVersion":1,"frameId":0,"timestampMs":${System.currentTimeMillis()},"commands":[${commands.joinToString(",")}]}"""
  }
}
