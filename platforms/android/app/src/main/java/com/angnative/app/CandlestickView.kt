/**
 * CandlestickView.kt — native Canvas renderer for the `na-candlestick`
 * contract element.
 *
 * The Angular renderer sends OHLC bars as a JSON **string** property
 * (`attr:bars`) because Android's JSONObject.optString cannot read a
 * JSONArray from ViewProps.properties. This view parses that string and
 * draws deterministic candles; it never fetches data or creates a WebView.
 */
package com.angnative.app

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.RectF
import android.util.AttributeSet
import android.view.View
import org.json.JSONObject

class CandlestickView(context: Context, attrs: AttributeSet? = null) :
  View(context, attrs) {

  data class Bar(
    val t: Long,
    val open: Double,
    val high: Double,
    val low: Double,
    val close: Double,
    val volume: Long,
  )

  private val bars = mutableListOf<Bar>()
  private val density = context.resources.displayMetrics.density
  private val padding = (10f * density)

  private val gridPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    color = Color.parseColor("#E2E8F0")
    strokeWidth = 1f * density
  }
  private val labelPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    color = Color.parseColor("#64748B")
    textSize = 10f * density
  }
  private val wickPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    strokeWidth = 1.5f * density
  }
  private val upPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    color = Color.parseColor("#16A34A")
    style = Paint.Style.FILL
  }
  private val downPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    color = Color.parseColor("#DC2626")
    style = Paint.Style.FILL
  }

  init {
    setWillNotDraw(false)
    setBackgroundColor(Color.WHITE)
  }

  /** Replace all bars only when the whole payload parses; otherwise keep the last good state. */
  fun setBars(json: String) {
    val parsed = try {
      val array = JSONObject(json).optJSONArray("bars")
        ?: throw IllegalArgumentException("missing bars array")
      val next = mutableListOf<Bar>()
      for (i in 0 until array.length()) {
        val item = array.getJSONObject(i)
        val bar = Bar(
          t = item.optLong("t"),
          open = item.optDouble("o"),
          high = item.optDouble("h"),
          low = item.optDouble("l"),
          close = item.optDouble("c"),
          volume = item.optLong("v"),
        )
        if (listOf(bar.open, bar.high, bar.low, bar.close).any { it.isNaN() || it.isInfinite() }) {
          throw IllegalArgumentException("non-finite OHLC value at index $i")
        }
        if (bar.high < bar.low) throw IllegalArgumentException("high < low at index $i")
        next.add(bar)
      }
      next
    } catch (e: Exception) {
      // Explicitly do not fabricate or partially apply chart data.
      return
    }
    bars.clear()
    bars.addAll(parsed)
    invalidate()
  }

  /** Test/diagnostics hook: number of currently drawable bars. */
  fun barCount(): Int = bars.size

  override fun onDraw(canvas: Canvas) {
    super.onDraw(canvas)
    if (bars.isEmpty()) return

    val left = padding
    val right = width - padding
    val top = padding
    val bottom = height - padding
    if (right <= left || bottom <= top) return

    val minPrice = bars.minOf { it.low }
    val maxPrice = bars.maxOf { it.high }
    val span = if (maxPrice > minPrice) maxPrice - minPrice else 1.0

    fun yOf(price: Double): Float =
      (bottom - ((price - minPrice) / span) * (bottom - top)).toFloat()

    // Four horizontal grid lines with price labels.
    for (i in 0..4) {
      val y = top + (bottom - top) * (i / 4f)
      canvas.drawLine(left, y, right, y, gridPaint)
      val labelPrice = maxPrice - span * (i / 4f)
      canvas.drawText(String.format("%.2f", labelPrice), left + 2f * density, y - 2f * density, labelPaint)
    }

    val spacing = (right - left) / bars.size
    val bodyHalfWidth = minOf(spacing * 0.3f, 8f * density).coerceAtLeast(1f)
    for ((index, bar) in bars.withIndex()) {
      val x = left + spacing * (index + 0.5f)
      val rising = bar.close >= bar.open
      wickPaint.color = if (rising) upPaint.color else downPaint.color
      canvas.drawLine(x, yOf(bar.high), x, yOf(bar.low), wickPaint)

      val bodyTop = yOf(maxOf(bar.open, bar.close))
      val bodyBottom = yOf(minOf(bar.open, bar.close))
      val rect = RectF(x - bodyHalfWidth, bodyTop, x + bodyHalfWidth, bodyBottom)
      if (rect.height() < 1.5f * density) {
        rect.top -= density
        rect.bottom += density
      }
      canvas.drawRect(rect, if (rising) upPaint else downPaint)
    }
  }
}
