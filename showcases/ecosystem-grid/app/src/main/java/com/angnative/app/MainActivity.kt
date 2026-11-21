/**
 * MainActivity.kt — AngNative Phase 2 ecosystem showcase: NATIVE data grid.
 *
 * This is the AG Grid-compatible NATIVE EQUIVALENT. AG Grid is a DOM/CSS/JS
 * grid and cannot execute on AngNative's native renderer; see Employee.kt.
 * The grid below is a real android RecyclerView (Adapter + ViewHolder +
 * recycling over 10,000 rows) — never a WebView, Capacitor or embedded browser.
 *
 * The Rust host bridge (JniBridge -> angnative-host-ffi via JNI) is kept intact
 * and is exercised at startup; its live status is displayed in the header.
 */
package com.angnative.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.util.Log
import android.util.TypedValue
import android.view.Gravity
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.WindowCompat
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView

class MainActivity : AppCompatActivity() {

  private lateinit var recycleCounter: TextView
  private lateinit var positionCounter: TextView
  private lateinit var bridgeStatus: TextView
  private var gridAdapter: EmployeeAdapter? = null
  private var hostBridge: JniBridge? = null

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    WindowCompat.setDecorFitsSystemWindows(window, true)
    supportActionBar?.hide()

    val employees = EmployeeData.generate()
    gridAdapter = EmployeeAdapter(employees) { count ->
      // onViewRecycled runs on the main thread; update badge in place.
      recycleCounter.text = count.toString()
    }

    val root = LinearLayout(this).apply {
      orientation = LinearLayout.VERTICAL
      setBackgroundColor(Color.parseColor("#F1F5F9"))
      layoutParams = FrameLayout.LayoutParams(
        FrameLayout.LayoutParams.MATCH_PARENT,
        FrameLayout.LayoutParams.MATCH_PARENT
      )
    }

    root.addView(buildHeader(employees.size))
    // Header view now exists, so the live bridge status can be displayed.
    // The Rust host bridge itself is kept intact and genuinely exercised.
    initRustHost()
    root.addView(
      buildGrid(),
      LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, 0, 1f).apply { topMargin = dp(12) }
    )
    root.addView(buildFooter())

    setContentView(root)
  }

  /** Header: title, row count, live recycle counter and Rust bridge status. */
  private fun buildHeader(rowCount: Int): LinearLayout {
    val header = LinearLayout(this).apply {
      orientation = LinearLayout.VERTICAL
      setPadding(dp(16), dp(16), dp(16), dp(12))
      background = cardBackground("#FFFFFF")
    }

    header.addView(
      label(
        "ANGNATIVE • ECOSYSTEM SHOWCASE (NATIVE REIMPLEMENTATION)",
        11f, Color.parseColor("#4338CA"), bold = true
      ).apply {
        setPadding(0, 0, 0, dp(4))
      }
    )
    header.addView(
      label("Native Virtualized Data Grid", 24f, Color.parseColor("#0F172A"), bold = true)
    )
    header.addView(
      label(
        "AG Grid-compatible native equivalent — 10,000 rows, real RecyclerView virtualization, 0 WebViews.",
        13f, Color.parseColor("#64748B")
      ).apply { setPadding(0, dp(2), 0, dp(12)) }
    )

    // Three live stat tiles: the recycle + position counters make virtualization
    // observable while the user flings through 10,000 rows.
    val stats = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL }
    recycleCounter = TextView(this)
    positionCounter = TextView(this)
    stats.addView(statTile("ROWS", String.format(java.util.Locale.US, "%,d", rowCount), 1.1f))
    stats.addView(statTile("VIEWS RECYCLED", "0", 1.5f, recycleCounter))
    stats.addView(statTile("TOP POSITION", "0", 1.4f, positionCounter))
    header.addView(stats)

    bridgeStatus = label("Rust host bridge: initializing…", 11f, Color.parseColor("#475569"))
    bridgeStatus.setPadding(0, dp(10), 0, 0)
    header.addView(bridgeStatus)
    return header
  }

  /** Column header + the genuinely virtualized RecyclerView. */
  private fun buildGrid(): LinearLayout {
    val gridCard = LinearLayout(this).apply {
      orientation = LinearLayout.VERTICAL
      setPadding(0, 0, 0, 0)
      background = cardBackground("#FFFFFF")
      clipChildren = true
    }

    val columnHeader = GridRowView.build(this, header = true)
    gridCard.addView(
      columnHeader,
      LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(GridRowView.HEADER_HEIGHT_DP))
    )

    // Separator under the header row.
    val separator = android.view.View(this).apply {
      setBackgroundColor(Color.parseColor("#CBD5E1"))
    }
    gridCard.addView(separator, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(1)))

    val recyclerView = RecyclerView(this).apply {
      layoutManager = LinearLayoutManager(context)
      adapter = gridAdapter
      // Stable row height (GridRowView.ROW_HEIGHT_DP) makes this safe + fast.
      setHasFixedSize(true)
      setItemViewCacheSize(8)
      recycledViewPool.setMaxRecycledViews(0, 16)
      overScrollMode = RecyclerView.OVER_SCROLL_ALWAYS
    }
    recyclerView.addOnScrollListener(object : RecyclerView.OnScrollListener() {
      override fun onScrolled(rv: RecyclerView, dx: Int, dy: Int) {
        val first = (rv.layoutManager as? LinearLayoutManager)
          ?.findFirstVisibleItemPosition() ?: 0
        positionCounter.text = first.toString()
      }
    })
    gridCard.addView(recyclerView, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, 0, 1f))
    return gridCard
  }

  private fun buildFooter(): TextView = label(
    "AngNative Runtime v0.1.0 • Rust FFI / JNI • NATIVE views only",
    11f, Color.parseColor("#94A3B8")
  ).apply {
    gravity = Gravity.CENTER
    setPadding(0, dp(10), 0, dp(12))
  }

  /**
   * Exercise the real Rust host through JNI. The native host, ViewFactory
   * registration and C ABI are untouched from the verified material template.
   */
  private fun initRustHost() {
    try {
      AngNativeApp.initialize(applicationContext)
      val bridge = JniBridge(this)
      bridge.initHost()
      bridge.processBatch(buildBatch {
        createNode(1, "View")
        createNode(2, "Text")
        setText(2, "native grid")
        insertChild(1, 2, 0)
      })
      val nodes = bridge.nodeCount()
      bridgeStatus.text = "Rust host bridge: ONLINE • shadow nodes: $nodes • WebViews: 0"
      bridgeStatus.setTextColor(Color.parseColor("#0F766E"))
      hostBridge = bridge
      Log.i("AngNative", "Grid showcase Rust host online, nodes=$nodes")
    } catch (t: Throwable) {
      Log.e("AngNative", "Rust host bridge init failed", t)
      bridgeStatus.text = "Rust host bridge: OFFLINE (${t.javaClass.simpleName})"
      bridgeStatus.setTextColor(Color.parseColor("#B91C1C"))
    }
  }

  private fun statTile(
    caption: String,
    value: String,
    weight: Float,
    valueView: TextView? = null
  ): LinearLayout {
    val tile = LinearLayout(this).apply {
      orientation = LinearLayout.VERTICAL
      gravity = Gravity.CENTER
      setPadding(dp(8), dp(10), dp(8), dp(10))
      layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, weight)
      background = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = dp(10).toFloat()
        setColor(Color.parseColor("#F1F5F9"))
      }
    }
    val number = (valueView ?: TextView(this)).apply {
      text = value
      gravity = Gravity.CENTER
      setTextSize(TypedValue.COMPLEX_UNIT_SP, 18f)
      setTextColor(Color.parseColor("#0F172A"))
      typeface = Typeface.DEFAULT_BOLD
    }
    val cap = label(caption, 9f, Color.parseColor("#64748B"), bold = true)
    cap.gravity = Gravity.CENTER
    tile.addView(number)
    tile.addView(cap)
    return tile
  }

  private fun label(text: String, sizeSp: Float, color: Int, bold: Boolean = false): TextView =
    TextView(this).apply {
      this.text = text
      setTextSize(TypedValue.COMPLEX_UNIT_SP, sizeSp)
      setTextColor(color)
      typeface = if (bold) Typeface.DEFAULT_BOLD else Typeface.DEFAULT
      layoutParams = LinearLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT,
        ViewGroup.LayoutParams.WRAP_CONTENT
      )
    }

  private fun cardBackground(color: String): GradientDrawable = GradientDrawable().apply {
    shape = GradientDrawable.RECTANGLE
    cornerRadius = dp(14).toFloat()
    setColor(Color.parseColor(color))
  }

  private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()

  override fun onDestroy() {
    hostBridge?.destroyHost()
    hostBridge = null
    gridAdapter = null
    super.onDestroy()
  }
}
