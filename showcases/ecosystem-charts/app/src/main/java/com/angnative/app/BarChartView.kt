/**
 * BarChartView.kt — vertical bar chart rendered entirely with Canvas APIs.
 *
 * Gridlines, y-axis value labels, x-axis category labels, rounded bars
 * (LinearGradient + drawRoundRect) and the tap tooltip are all native drawing.
 */

package com.angnative.app

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.LinearGradient
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.Shader
import android.util.AttributeSet
import kotlin.math.ceil

class BarChartView @JvmOverloads constructor(
  context: Context,
  attrs: AttributeSet? = null
) : NativeChartView(context, attrs) {

  private val barPaint = Paint(Paint.ANTI_ALIAS_FLAG)
  private val selectionPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    style = Paint.Style.STROKE
    color = Color.parseColor("#0F172A")
    strokeWidth = dp(2f)
  }
  private var bars: List<RectF> = emptyList()

  override fun drawChart(canvas: Canvas, spec: ChartSpec, area: RectF) {
    if (spec.entries.isEmpty() || area.width() <= 0 || area.height() <= 0) return

    // 15% headroom keeps the value printed above the tallest bar inside the plot.
    val ceiling = ceil(spec.max * 1.15).coerceAtLeast(1.0)
    drawGrid(canvas, area, ceiling)

    val slot = area.width() / spec.entries.size
    val barWidth = minOf(slot * 0.58f, dp(56f))
    val corner = dp(6f)
    val newBars = ArrayList<RectF>(spec.entries.size)

    spec.entries.forEachIndexed { index, entry ->
      val centerX = area.left + slot * (index + 0.5f)
      val barHeight = (entry.value / ceiling).toFloat() * area.height()
      val rect = RectF(
        centerX - barWidth / 2f,
        area.bottom - barHeight,
        centerX + barWidth / 2f,
        area.bottom
      )
      newBars.add(rect)

      val topColor = lighten(entry.color, 0.38f)
      barPaint.shader = LinearGradient(
        rect.left, rect.top, rect.left, rect.bottom,
        topColor, entry.color, Shader.TileMode.CLAMP
      )
      canvas.drawRoundRect(rect, corner, corner, barPaint)
      if (index == selectedIndex) {
        canvas.drawRoundRect(rect, corner, corner, selectionPaint)
      }

      canvas.drawText(
        ChartCatalog.format(entry.value),
        centerX, rect.top - dp(8f), valuePaint
      )
      canvas.drawText(entry.label, centerX, area.bottom + dp(24f), xLabelPaint)
    }
    bars = newBars
  }

  override fun hitTest(x: Float, y: Float): Hit? {
    val slack = dp(6f)
    for (index in bars.indices) {
      val rect = bars[index]
      if (x >= rect.left - slack && x <= rect.right + slack &&
        y >= rect.top - dp(20f) && y <= rect.bottom + dp(30f)
      ) {
        return Hit(index, rect.centerX(), rect.top)
      }
    }
    return null
  }

  /** Blend a colour toward white for the top stop of the bar gradient. */
  private fun lighten(color: Int, amount: Float): Int {
    fun mix(channel: Int): Int =
      (channel + (255 - channel) * amount).toInt().coerceIn(0, 255)
    return Color.rgb(mix(Color.red(color)), mix(Color.green(color)), mix(Color.blue(color)))
  }
}
