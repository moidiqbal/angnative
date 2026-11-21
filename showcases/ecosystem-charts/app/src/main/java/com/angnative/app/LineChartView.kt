/**
 * LineChartView.kt — line + area chart built from native Canvas Paths.
 *
 * The trend line is a Path drawn with drawPath, the shaded area is a second
 * closed Path filled with a LinearGradient shader, and every datum is a
 * drawCircle point. No browser canvas and no charting dependency is involved.
 */

package com.angnative.app

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.LinearGradient
import android.graphics.Paint
import android.graphics.Path
import android.graphics.PointF
import android.graphics.RectF
import android.graphics.Shader
import android.util.AttributeSet
import kotlin.math.ceil

class LineChartView @JvmOverloads constructor(
  context: Context,
  attrs: AttributeSet? = null
) : NativeChartView(context, attrs) {

  private val areaPaint = Paint(Paint.ANTI_ALIAS_FLAG)
  private val linePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    style = Paint.Style.STROKE
    strokeWidth = dp(3f)
    strokeCap = Paint.Cap.ROUND
    strokeJoin = Paint.Join.ROUND
  }
  private val pointFillPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    color = Color.WHITE
    style = Paint.Style.FILL
  }
  private val pointStrokePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    style = Paint.Style.STROKE
    strokeWidth = dp(2.5f)
  }
  private val selectedHaloPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    style = Paint.Style.STROKE
    color = Color.parseColor("#0F172A")
    strokeWidth = dp(2f)
  }
  private var points: List<PointF> = emptyList()

  override fun drawChart(canvas: Canvas, spec: ChartSpec, area: RectF) {
    val entries = spec.entries
    if (entries.isEmpty() || area.width() <= 0 || area.height() <= 0) return

    val color = entries.first().color
    val ceiling = ceil(spec.max * 1.15).coerceAtLeast(1.0)
    drawGrid(canvas, area, ceiling)

    val stepX = if (entries.size > 1) area.width() / (entries.size - 1) else 0f
    points = entries.mapIndexed { index, entry ->
      PointF(
        area.left + stepX * index,
        area.bottom - (entry.value / ceiling).toFloat() * area.height()
      )
    }

    // Closed path for the shaded area under the trend line.
    val fillPath = Path()
    fillPath.moveTo(points.first().x, area.bottom)
    points.forEach { fillPath.lineTo(it.x, it.y) }
    fillPath.lineTo(points.last().x, area.bottom)
    fillPath.close()
    areaPaint.shader = LinearGradient(
      0f, area.top, 0f, area.bottom,
      withAlpha(color, 64), withAlpha(color, 0), Shader.TileMode.CLAMP
    )
    canvas.drawPath(fillPath, areaPaint)

    // The trend line itself.
    val trend = Path()
    trend.moveTo(points.first().x, points.first().y)
    for (i in 1 until points.size) trend.lineTo(points[i].x, points[i].y)
    linePaint.color = color
    canvas.drawPath(trend, linePaint)

    points.forEachIndexed { index, point ->
      if (index == selectedIndex) {
        canvas.drawCircle(point.x, point.y, dp(11f), selectedHaloPaint)
      }
      val radius = if (index == selectedIndex) dp(7f) else dp(5f)
      canvas.drawCircle(point.x, point.y, radius, pointFillPaint)
      pointStrokePaint.color = color
      canvas.drawCircle(point.x, point.y, radius, pointStrokePaint)

      canvas.drawText(
        ChartCatalog.format(entries[index].value),
        point.x, point.y - dp(13f), valuePaint
      )
      canvas.drawText(entries[index].label, point.x, area.bottom + dp(24f), xLabelPaint)
    }
  }

  override fun hitTest(x: Float, y: Float): Hit? {
    var bestIndex = -1
    var bestDistance = Float.MAX_VALUE
    points.forEachIndexed { index, point ->
      val dx = kotlin.math.abs(x - point.x)
      val dy = kotlin.math.abs(y - point.y)
      if (dx < dp(40f) && dy < dp(80f) && dx + dy < bestDistance) {
        bestDistance = dx + dy
        bestIndex = index
      }
    }
    return if (bestIndex >= 0) {
      val point = points[bestIndex]
      Hit(bestIndex, point.x, point.y)
    } else null
  }

  private fun withAlpha(color: Int, alpha: Int): Int =
    Color.argb(alpha, Color.red(color), Color.green(color), Color.blue(color))
}
