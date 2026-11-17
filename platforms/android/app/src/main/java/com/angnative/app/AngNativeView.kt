/**
 * AngNativeView.kt — Native container for AngNative on Android.
 *
 * Demonstrates the AngNative native host bridge with Rust (C ABI)
 * and renders high-performance native Android UI widgets.
 */

package com.angnative.app

import android.animation.AnimatorSet
import android.animation.ObjectAnimator
import android.content.Context
import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.graphics.drawable.RippleDrawable
import android.util.AttributeSet
import android.util.Log
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.animation.OvershootInterpolator
import android.widget.*
import androidx.core.widget.NestedScrollView

fun buildBatch(block: BatchBuilder.() -> Unit): String = BatchBuilder().apply(block).build()

class AngNativeView(context: Context, attrs: AttributeSet? = null) :
  FrameLayout(context, attrs) {

  private val tag = "AngNative"
  private lateinit var jniBridge: JniBridge
  private var clickCount = 0

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
   * Dispatches command batches to the Rust host (angnative-host-ffi)
   * and renders the native Android views with click interaction.
   */
  fun loadHelloWorldApp() {
    removeAllViews()

    // 1. Send initial command batch to the Rust host
    val initialBatch = buildBatch {
      // Root container (id=1)
      createNode(1, "View")
      setStyle(1, "flexDirection", "column")
      setStyle(1, "padding", "24px")

      // Badge (id=2)
      createNode(2, "Text")
      setText(2, "ANGNATIVE • ANDROID 17")
      setStyle(2, "fontSize", "11px")
      setStyle(2, "fontWeight", "bold")
      insertChild(1, 2, 0)

      // Title (id=3)
      createNode(3, "Text")
      setText(3, "Hello World Demo")
      setStyle(3, "fontSize", "26px")
      setStyle(3, "fontWeight", "bold")
      insertChild(1, 3, 1)

      // Subtitle (id=4)
      createNode(4, "Text")
      setText(4, "Tap the button below to display Hello World via the AngNative bridge.")
      setStyle(4, "fontSize", "14px")
      setStyle(4, "color", "#64748B")
      insertChild(1, 4, 2)

      // Button (id=5)
      createNode(5, "Button")
      setText(5, "Click Me!")
      insertChild(1, 5, 3)

      // Output text (id=6)
      createNode(6, "Text")
      setText(6, "")
      insertChild(1, 6, 4)
    }

    val initResponse = jniBridge.processBatch(initialBatch)
    Log.i(tag, "Initial batch dispatched to Rust host. Result: $initResponse")
    Log.i(tag, "Total nodes in Rust shadow tree: ${jniBridge.nodeCount()}")

    // 2. Build modern native Android UI
    val density = resources.displayMetrics.density
    fun toPx(dp: Int): Int = (dp * density).toInt()

    val scrollView = NestedScrollView(context).apply {
      layoutParams = LayoutParams(
        LayoutParams.MATCH_PARENT,
        LayoutParams.MATCH_PARENT
      )
      isFillViewport = true
    }

    val contentLayout = LinearLayout(context).apply {
      orientation = LinearLayout.VERTICAL
      layoutParams = LinearLayout.LayoutParams(
        LinearLayout.LayoutParams.MATCH_PARENT,
        LinearLayout.LayoutParams.WRAP_CONTENT
      )
      // Clean top padding starting directly from the top
      setPadding(toPx(16), toPx(16), toPx(16), toPx(24))
      gravity = Gravity.TOP
    }

    // Main Card Container
    val cardLayout = LinearLayout(context).apply {
      orientation = LinearLayout.VERTICAL
      layoutParams = LinearLayout.LayoutParams(
        LinearLayout.LayoutParams.MATCH_PARENT,
        LinearLayout.LayoutParams.WRAP_CONTENT
      ).apply {
        topMargin = toPx(4)
      }
      val cardBg = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = density * 18
        setColor(Color.WHITE)
        setStroke(toPx(1), Color.parseColor("#E2E8F0"))
      }
      background = cardBg
      elevation = density * 4
      setPadding(toPx(20), toPx(22), toPx(20), toPx(22))
    }

    // Badge Pill
    val badgeView = TextView(context).apply {
      text = "🚀 ANGNATIVE • ANDROID 17"
      textSize = 11f
      typeface = Typeface.DEFAULT_BOLD
      setTextColor(Color.parseColor("#4338CA"))
      val badgeBg = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = density * 12
        setColor(Color.parseColor("#EEF2FF"))
      }
      background = badgeBg
      setPadding(toPx(12), toPx(6), toPx(12), toPx(6))
      layoutParams = LinearLayout.LayoutParams(
        LinearLayout.LayoutParams.WRAP_CONTENT,
        LinearLayout.LayoutParams.WRAP_CONTENT
      )
    }
    cardLayout.addView(badgeView)

    // Title
    val titleView = TextView(context).apply {
      text = "Hello World Demo"
      textSize = 26f
      typeface = Typeface.DEFAULT_BOLD
      setTextColor(Color.parseColor("#0F172A"))
      layoutParams = LinearLayout.LayoutParams(
        LinearLayout.LayoutParams.WRAP_CONTENT,
        LinearLayout.LayoutParams.WRAP_CONTENT
      ).apply {
        topMargin = toPx(16)
      }
    }
    cardLayout.addView(titleView)

    // Subtitle
    val subtitleView = TextView(context).apply {
      text = "Click the button below to dispatch native events and display Hello World via the AngNative Rust host."
      textSize = 14f
      setTextColor(Color.parseColor("#64748B"))
      setLineSpacing(0f, 1.25f)
      layoutParams = LinearLayout.LayoutParams(
        LinearLayout.LayoutParams.WRAP_CONTENT,
        LinearLayout.LayoutParams.WRAP_CONTENT
      ).apply {
        topMargin = toPx(8)
      }
    }
    cardLayout.addView(subtitleView)

    // Interactive Button
    val button = Button(context).apply {
      text = "Show Hello World"
      textSize = 16f
      typeface = Typeface.DEFAULT_BOLD
      setTextColor(Color.WHITE)
      isAllCaps = false

      val normalBg = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = density * 12
        setColor(Color.parseColor("#2563EB"))
      }
      val rippleColor = ColorStateList.valueOf(Color.parseColor("#1D4ED8"))
      background = RippleDrawable(rippleColor, normalBg, null)
      elevation = density * 4

      layoutParams = LinearLayout.LayoutParams(
        LinearLayout.LayoutParams.MATCH_PARENT,
        toPx(54)
      ).apply {
        topMargin = toPx(28)
      }
    }
    cardLayout.addView(button)

    // Result Card Box
    val resultBox = LinearLayout(context).apply {
      orientation = LinearLayout.VERTICAL
      gravity = Gravity.CENTER
      layoutParams = LinearLayout.LayoutParams(
        LinearLayout.LayoutParams.MATCH_PARENT,
        LinearLayout.LayoutParams.WRAP_CONTENT
      ).apply {
        topMargin = toPx(24)
      }
      val boxBg = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = density * 14
        setColor(Color.parseColor("#F1F5F9"))
        setStroke(toPx(1), Color.parseColor("#CBD5E1"))
      }
      background = boxBg
      setPadding(toPx(20), toPx(24), toPx(20), toPx(24))
    }

    val resultTextView = TextView(context).apply {
      text = "Tap button above to display"
      textSize = 15f
      typeface = Typeface.DEFAULT
      setTextColor(Color.parseColor("#94A3B8"))
      gravity = Gravity.CENTER
    }
    resultBox.addView(resultTextView)

    val infoTextView = TextView(context).apply {
      text = ""
      textSize = 12f
      typeface = Typeface.DEFAULT_BOLD
      setTextColor(Color.parseColor("#16A34A"))
      gravity = Gravity.CENTER
      layoutParams = LinearLayout.LayoutParams(
        LinearLayout.LayoutParams.WRAP_CONTENT,
        LinearLayout.LayoutParams.WRAP_CONTENT
      ).apply {
        topMargin = toPx(8)
      }
      visibility = View.GONE
    }
    resultBox.addView(infoTextView)

    cardLayout.addView(resultBox)

    // Secondary Reset Button
    val resetButton = Button(context).apply {
      text = "Reset"
      textSize = 14f
      typeface = Typeface.DEFAULT_BOLD
      setTextColor(Color.parseColor("#64748B"))
      isAllCaps = false
      val resetBg = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = density * 10
        setColor(Color.parseColor("#F1F5F9"))
        setStroke(toPx(1), Color.parseColor("#E2E8F0"))
      }
      background = resetBg
      layoutParams = LinearLayout.LayoutParams(
        LinearLayout.LayoutParams.MATCH_PARENT,
        toPx(44)
      ).apply {
        topMargin = toPx(16)
      }
    }
    cardLayout.addView(resetButton)

    // Setup Click Listeners
    button.setOnClickListener {
      clickCount++

      // 1. Update text node in the Rust host shadow tree
      val textValue = if (clickCount == 1) "Hello World!" else "Hello World! (x$clickCount)"
      val updateBatch = buildBatch {
        setText(6, textValue)
      }
      val rustResponse = jniBridge.processBatch(updateBatch)
      Log.i(tag, "Dispatched SetTextContent to Rust host: $rustResponse")

      // 2. Update Android UI
      resultTextView.apply {
        text = "🎉 Hello World! 🎉"
        textSize = 24f
        typeface = Typeface.DEFAULT_BOLD
        setTextColor(Color.parseColor("#1D4ED8"))

        // Pop & Bounce Animation
        val scaleX = ObjectAnimator.ofFloat(this, "scaleX", 0.7f, 1.1f, 1.0f)
        val scaleY = ObjectAnimator.ofFloat(this, "scaleY", 0.7f, 1.1f, 1.0f)
        val alpha = ObjectAnimator.ofFloat(this, "alpha", 0.2f, 1.0f)
        AnimatorSet().apply {
          playTogether(scaleX, scaleY, alpha)
          duration = 350
          interpolator = OvershootInterpolator(1.8f)
          start()
        }
      }

      val boxBgActive = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = density * 14
        setColor(Color.parseColor("#EFF6FF"))
        setStroke(toPx(2), Color.parseColor("#3B82F6"))
      }
      resultBox.background = boxBgActive

      infoTextView.apply {
        text = "✓ Event Synced with Rust Host • Click #$clickCount"
        visibility = View.VISIBLE
      }

      Toast.makeText(context, "Hello World! (AngNative)", Toast.LENGTH_SHORT).show()
    }

    resetButton.setOnClickListener {
      clickCount = 0

      // Reset node in Rust host
      val resetBatch = buildBatch {
        setText(6, "")
      }
      jniBridge.processBatch(resetBatch)

      resultTextView.apply {
        text = "Tap button above to display"
        textSize = 15f
        typeface = Typeface.DEFAULT
        setTextColor(Color.parseColor("#94A3B8"))
      }

      val boxBgDefault = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = density * 14
        setColor(Color.parseColor("#F1F5F9"))
        setStroke(toPx(1), Color.parseColor("#CBD5E1"))
      }
      resultBox.background = boxBgDefault
      infoTextView.visibility = View.GONE
    }

    contentLayout.addView(cardLayout)

    // Footer info
    val footerView = TextView(context).apply {
      text = "AngNative Runtime v0.1.0 • Rust FFI • Taaffy Layout"
      textSize = 12f
      setTextColor(Color.parseColor("#94A3B8"))
      gravity = Gravity.CENTER
      layoutParams = LinearLayout.LayoutParams(
        LinearLayout.LayoutParams.WRAP_CONTENT,
        LinearLayout.LayoutParams.WRAP_CONTENT
      ).apply {
        topMargin = toPx(28)
      }
    }
    contentLayout.addView(footerView)

    scrollView.addView(contentLayout)
    addView(scrollView)
  }

  /** Alias to load the demo app. */
  fun loadDemoApp() {
    loadHelloWorldApp()
  }

  fun onResume() {}
  fun onPause() {}

  fun destroy() {
    jniBridge.destroyHost()
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
