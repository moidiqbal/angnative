/**
 * MainActivity.kt — entry point for the AngNative Native Calendar Showcase.
 *
 * The Rust JNI bridge from the material template is kept intact and is
 * exercised at startup (host create + a command batch), but the calendar
 * itself is NOT produced by that bridge: the 7-column month grid is a custom
 * Android View painted with Canvas/Paint. FullCalendar is a DOM/JS library and
 * cannot run unchanged natively — see the honesty note in CalendarEvent.kt.
 * Zero WebView.
 */

package com.angnative.app

import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.util.Log
import android.view.Gravity
import android.view.ViewGroup
import android.widget.Button
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.WindowCompat

class MainActivity : AppCompatActivity() {

  private lateinit var monthTitle: TextView
  private lateinit var grid: MonthGridView
  private lateinit var detailsTitle: TextView
  private lateinit var detailsList: LinearLayout
  private lateinit var statusText: TextView
  private lateinit var bridgeStatus: String
  private var bridge: JniBridge? = null

  private val density: Float get() = resources.displayMetrics.density
  private fun dp(value: Float): Int = (value * density).toInt()
  private fun dpf(value: Float): Float = value * density

  private fun cardBackground(): GradientDrawable = GradientDrawable().apply {
    shape = GradientDrawable.RECTANGLE
    cornerRadius = dpf(14f)
    setColor(Color.WHITE)
    setStroke(dp(1f), Color.parseColor("#E2E8F0"))
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)

    WindowCompat.setDecorFitsSystemWindows(window, true)
    supportActionBar?.hide()

    // Keep the template's Rust host alive and prove the JNI path still works.
    // Calendar pixels never pass through this bridge or a WebView.
    bridgeStatus = initializeRustBridge()

    setContentView(buildUi())
    refresh(grid.selectedDate())
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
      "Rust host unavailable (${e.javaClass.simpleName}) — calendar still renders natively"
    }
  }

  private fun buildUi(): LinearLayout {
    val root = LinearLayout(this).apply {
      orientation = LinearLayout.VERTICAL
      setBackgroundColor(Color.parseColor("#F8FAFC"))
    }

    root.addView(TextView(this).apply {
      text = "ANGNATIVE  •  ECOSYSTEM SHOWCASE"
      textSize = 10f
      setTextColor(Color.parseColor("#4338CA"))
      typeface = android.graphics.Typeface.DEFAULT_BOLD
      setPadding(dp(18f), dp(16f), dp(18f), dp(2f))
    })

    root.addView(TextView(this).apply {
      text = "Native Month Calendar"
      textSize = 22f
      setTextColor(Color.parseColor("#0F172A"))
      typeface = android.graphics.Typeface.DEFAULT_BOLD
      setPadding(dp(18f), 0, dp(18f), dp(6f))
    })

    // Month navigation row.
    val nav = LinearLayout(this).apply {
      orientation = LinearLayout.HORIZONTAL
      layoutParams = LinearLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT
      ).apply { setMargins(dp(16f), 0, dp(16f), 0) }
    }
    val prev = Button(this).apply {
      text = "‹ Prev"
      isAllCaps = false
      textSize = 13f
      layoutParams = LinearLayout.LayoutParams(dp(84f), dp(44f))
      setOnClickListener { grid.shiftMonth(-1) }
    }
    monthTitle = TextView(this).apply {
      textSize = 18f
      gravity = Gravity.CENTER
      setTextColor(Color.parseColor("#0F172A"))
      typeface = android.graphics.Typeface.DEFAULT_BOLD
      layoutParams = LinearLayout.LayoutParams(0, dp(44f), 1f)
    }
    val next = Button(this).apply {
      text = "Next ›"
      isAllCaps = false
      textSize = 13f
      layoutParams = LinearLayout.LayoutParams(dp(84f), dp(44f))
      setOnClickListener { grid.shiftMonth(1) }
    }
    nav.addView(prev)
    nav.addView(monthTitle)
    nav.addView(next)
    root.addView(nav)

    // The native month grid lives in a simple card container.
    grid = MonthGridView(this).apply { onDateSelected = { refresh(it) } }
    val gridCard = FrameLayout(this).apply {
      background = cardBackground()
      elevation = dpf(3f)
      setPadding(dp(6f), dp(6f), dp(6f), dp(6f))
      layoutParams = LinearLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT
      ).apply { setMargins(dp(16f), dp(12f), dp(16f), 0) }
    }
    gridCard.addView(
      grid,
      FrameLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT,
        ViewGroup.LayoutParams.WRAP_CONTENT
      )
    )
    root.addView(gridCard)

    // Details panel: selected date + that day's events.
    val detailsCard = LinearLayout(this).apply {
      orientation = LinearLayout.VERTICAL
      background = cardBackground()
      elevation = dpf(2f)
      setPadding(dp(14f), dp(12f), dp(14f), dp(12f))
      layoutParams = LinearLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT
      ).apply { setMargins(dp(16f), dp(12f), dp(16f), 0) }
    }
    detailsTitle = TextView(this).apply {
      textSize = 16f
      setTextColor(Color.parseColor("#0F172A"))
      typeface = android.graphics.Typeface.DEFAULT_BOLD
    }
    detailsList = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
    detailsCard.addView(detailsTitle)
    detailsCard.addView(detailsList)
    root.addView(detailsCard)

    root.addView(TextView(this).apply {
      text = "Tap any day to select it  •  ‹ / › change month  •  blue outline = today"
      textSize = 11f
      setTextColor(Color.parseColor("#64748B"))
      gravity = Gravity.CENTER
      setPadding(dp(16f), dp(10f), dp(16f), 0)
    })

    statusText = TextView(this).apply {
      textSize = 10f
      setTextColor(Color.parseColor("#475569"))
      gravity = Gravity.CENTER
      setPadding(dp(16f), dp(6f), dp(16f), 0)
    }
    root.addView(statusText)

    root.addView(TextView(this).apply {
      text = "HONESTY: FullCalendar is a DOM/JS library and cannot run unchanged " +
        "natively. This is a native reimplementation drawn with Android Canvas " +
        "and a custom View. Zero WebView / browser engine."
      textSize = 10f
      setTextColor(Color.parseColor("#94A3B8"))
      gravity = Gravity.CENTER
      setPadding(dp(16f), dp(8f), dp(16f), dp(14f))
    })

    return root
  }

  /** Update month title, selected-date header, and that day's event rows. */
  private fun refresh(date: MonthGridView.CellDate) {
    monthTitle.text = grid.displayedTitle()
    detailsTitle.text = CalendarData.fullDate(date.year, date.month, date.day)
    detailsList.removeAllViews()

    val events = CalendarData.eventsOn(date.year, date.month, date.day)
    if (events.isEmpty()) {
      detailsList.addView(TextView(this).apply {
        text = "No events on this day."
        textSize = 12f
        setTextColor(Color.parseColor("#94A3B8"))
        setPadding(0, dp(6f), 0, 0)
      })
    } else {
      events.forEach { event ->
        detailsList.addView(TextView(this).apply {
          text = "${event.time}  •  ${event.title}  [${event.category}]"
          textSize = 12f
          setTextColor(Color.parseColor("#475569"))
          setPadding(0, dp(4f), 0, 0)
        })
      }
    }
    statusText.text = "$bridgeStatus  •  WebViews: 0"
  }

  override fun onDestroy() {
    bridge?.destroyHost()
    bridge = null
    super.onDestroy()
  }

  companion object {
    private const val TAG = "AngNativeCalendar"
  }
}
