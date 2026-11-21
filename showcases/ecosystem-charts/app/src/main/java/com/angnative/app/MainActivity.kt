/**
 * MainActivity.kt — entry point for the AngNative Native Charts Showcase.
 *
 * The Rust JNI bridge from the material template is kept intact and is
 * initialized here (it still creates a host and processes a command batch),
 * but the visible charts are NOT produced by the bridge: they are custom
 * Android Views painting with Canvas. Apache ECharts cannot run natively —
 * see the honesty note in ChartData.kt.
 */

package com.angnative.app

import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.util.Log
import android.view.ViewGroup
import android.widget.Button
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.WindowCompat

class MainActivity : AppCompatActivity() {

  private lateinit var chartTitle: TextView
  private lateinit var chartDescription: TextView
  private lateinit var chartContainer: FrameLayout
  private lateinit var statusText: TextView
  private val tabs = mutableListOf<Button>()
  private var bridge: JniBridge? = null

  private val density: Float get() = resources.displayMetrics.density
  private fun dp(value: Float): Float = value * density

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)

    WindowCompat.setDecorFitsSystemWindows(window, true)
    supportActionBar?.hide()

    // Keep the template's Rust host alive and prove the JNI path still works.
    // Chart pixels themselves never pass through this bridge or a WebView.
    val bridgeStatus = initializeRustBridge()

    setContentView(buildUi(bridgeStatus))
    selectChart(0)
  }

  private fun initializeRustBridge(): String {
    return try {
      AngNativeApp.initialize(applicationContext)
      val host = JniBridge(applicationContext).also { bridge = it }
      host.initHost()
      val batch = buildBatch {
        createNode(1, "View")
        setStyle(1, "width", "100%")
      }
      val response = host.processBatch(batch)
      Log.i(TAG, "Rust host batch response: $response")
      "Rust host OK • ${host.nodeCount()} shadow nodes • JNI PlatformOps ready"
    } catch (e: Throwable) {
      Log.e(TAG, "Rust host initialization failed", e)
      "Rust host unavailable (${e.javaClass.simpleName}) — charts still render natively"
    }
  }

  private fun buildUi(bridgeStatus: String): LinearLayout {
    val root = LinearLayout(this).apply {
      orientation = LinearLayout.VERTICAL
      setBackgroundColor(Color.parseColor("#F8FAFC"))
    }

    root.addView(TextView(this).apply {
      text = "ANGNATIVE  •  ECOSYSTEM SHOWCASE"
      textSize = 10f
      setTextColor(Color.parseColor("#4338CA"))
      typeface = android.graphics.Typeface.DEFAULT_BOLD
      setPadding(dp(18f).toInt(), dp(16f).toInt(), dp(18f).toInt(), dp(2f).toInt())
    })

    root.addView(TextView(this).apply {
      text = "Native Chart Suite"
      textSize = 22f
      setTextColor(Color.parseColor("#0F172A"))
      typeface = android.graphics.Typeface.DEFAULT_BOLD
      setPadding(dp(18f).toInt(), dp(0f).toInt(), dp(18f).toInt(), dp(10f).toInt())
    })

    // Small header describing the active chart; updated on every tab change.
    val header = LinearLayout(this).apply {
      orientation = LinearLayout.VERTICAL
      background = cardBackground()
      elevation = dp(2f)
      setPadding(dp(16f).toInt(), dp(14f).toInt(), dp(16f).toInt(), dp(14f).toInt())
      layoutParams = LinearLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT
      ).apply { setMargins(dp(16f).toInt(), 0, dp(16f).toInt(), 0) }
    }
    chartTitle = TextView(this).apply {
      textSize = 17f
      setTextColor(Color.parseColor("#0F172A"))
      typeface = android.graphics.Typeface.DEFAULT_BOLD
    }
    chartDescription = TextView(this).apply {
      textSize = 12f
      setTextColor(Color.parseColor("#64748B"))
      setPadding(0, dp(5f).toInt(), 0, 0)
    }
    header.addView(chartTitle)
    header.addView(chartDescription)
    root.addView(header)

    val tabRow = LinearLayout(this).apply {
      orientation = LinearLayout.HORIZONTAL
      layoutParams = LinearLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT
      ).apply { setMargins(dp(16f).toInt(), dp(12f).toInt(), dp(16f).toInt(), 0) }
    }
    listOf("Bar", "Line", "Donut").forEachIndexed { index, label ->
      val button = Button(this).apply {
        text = label
        isAllCaps = false
        textSize = 13f
        layoutParams = LinearLayout.LayoutParams(0, dp(44f).toInt(), 1f).apply {
          setMargins(if (index == 0) 0 else dp(4f).toInt(), 0,
            if (index == 2) 0 else dp(4f).toInt(), 0)
        }
        setOnClickListener { selectChart(index) }
      }
      tabs.add(button)
      tabRow.addView(button)
    }
    root.addView(tabRow)

    chartContainer = FrameLayout(this).apply {
      background = cardBackground()
      elevation = dp(3f)
      minimumHeight = dp(240f).toInt()
      setPadding(dp(6f).toInt(), dp(6f).toInt(), dp(6f).toInt(), dp(6f).toInt())
      // Vertical LinearLayout: weight drives HEIGHT, so width must be MATCH_PARENT.
      layoutParams = LinearLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f
      ).apply {
        setMargins(dp(16f).toInt(), dp(12f).toInt(), dp(16f).toInt(), 0)
      }
    }
    root.addView(chartContainer)

    root.addView(TextView(this).apply {
      text = "Tap a bar / point / slice (or a legend row) to inspect its value."
      textSize = 11f
      setTextColor(Color.parseColor("#64748B"))
      gravity = android.view.Gravity.CENTER
      setPadding(dp(16f).toInt(), dp(10f).toInt(), dp(16f).toInt(), 0)
    })

    statusText = TextView(this).apply {
      text = "$bridgeStatus  •  WebViews: 0"
      textSize = 10f
      setTextColor(Color.parseColor("#475569"))
      gravity = android.view.Gravity.CENTER
      setPadding(dp(16f).toInt(), dp(6f).toInt(), dp(16f).toInt(), 0)
    }
    root.addView(statusText)

    root.addView(TextView(this).apply {
      text = "HONESTY: Apache ECharts / ng2-charts are DOM + browser-canvas " +
        "libraries and cannot run unchanged on a native renderer. These charts " +
        "are a native reimplementation drawn with Android Canvas. Zero WebView."
      textSize = 10f
      setTextColor(Color.parseColor("#94A3B8"))
      gravity = android.view.Gravity.CENTER
      setPadding(dp(16f).toInt(), dp(8f).toInt(), dp(16f).toInt(), dp(14f).toInt())
    })

    return root
  }

  private fun selectChart(index: Int) {
    val spec = ChartCatalog.charts[index]
    chartTitle.text = spec.title
    chartDescription.text = spec.description

    val chart: NativeChartView = when (spec.kind) {
      ChartKind.BAR -> BarChartView(this)
      ChartKind.LINE -> LineChartView(this)
      ChartKind.PIE -> PieChartView(this)
    }
    chart.setSpec(spec)
    chartContainer.removeAllViews()
    chartContainer.addView(
      chart,
      FrameLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT,
        ViewGroup.LayoutParams.MATCH_PARENT
      )
    )

    tabs.forEachIndexed { i, button ->
      val active = i == index
      button.background = tabBackground(active)
      button.setTextColor(if (active) Color.WHITE else Color.parseColor("#475569"))
    }
  }

  private fun cardBackground(): GradientDrawable = GradientDrawable().apply {
    shape = GradientDrawable.RECTANGLE
    cornerRadius = dp(14f)
    setColor(Color.WHITE)
    setStroke(dp(1f).toInt(), Color.parseColor("#E2E8F0"))
  }

  private fun tabBackground(active: Boolean): GradientDrawable = GradientDrawable().apply {
    shape = GradientDrawable.RECTANGLE
    cornerRadius = dp(10f)
    setColor(if (active) Color.parseColor("#2563EB") else Color.WHITE)
    setStroke(
      dp(1f).toInt(),
      Color.parseColor(if (active) "#2563EB" else "#CBD5E1")
    )
  }

  override fun onDestroy() {
    bridge?.destroyHost()
    bridge = null
    super.onDestroy()
  }

  companion object {
    private const val TAG = "AngNativeCharts"
  }
}
