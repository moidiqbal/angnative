/**
 * PieChartView.kt — donut chart made of native Path sectors.
 *
 * Each slice is an annular sector Path built from two Canvas arcs (outer and
 * inner), filled and stroked with Paint. The legend and centre summary are
 * ordinary drawText/drawRoundRect calls. A tap on a slice OR a legend row
 * selects a datum and raises the shared tooltip.
 */

package com.angnative.app

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.graphics.RectF
import android.graphics.Typeface
import android.util.AttributeSet
import kotlin.math.atan2
import kotlin.math.cos
import kotlin.math.min
import kotlin.math.sin

class PieChartView @JvmOverloads constructor(
  context: Context,
  attrs: AttributeSet? = null
) : NativeChartView(context, attrs) {

  private data class Slice(val start: Float, val sweep: Float)

  private val slicePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL }
  private val separationPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    style = Paint.Style.STROKE
    color = Color.WHITE
    strokeWidth = dp(2f)
  }
  private val selectedSlicePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    style = Paint.Style.STROKE
    color = Color.parseColor("#0F172A")
    strokeWidth = dp(2.5f)
  }
  private val swatchPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL }
  private val rowHighlightPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    style = Paint.Style.FILL
    color = Color.parseColor("#EEF2FF")
  }
  private val centerCaptionPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    color = Color.parseColor("#94A3B8")
    textSize = dp(10f)
    textAlign = Paint.Align.CENTER
  }
  private val centerValuePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    color = Color.parseColor("#0F172A")
    textSize = dp(20f)
    textAlign = Paint.Align.CENTER
    typeface = Typeface.DEFAULT_BOLD
  }
  private val legendLabelPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    color = Color.parseColor("#334155")
    textSize = dp(12f)
    textAlign = Paint.Align.LEFT
  }
  private val legendValuePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    color = Color.parseColor("#94A3B8")
    textSize = dp(11f)
    textAlign = Paint.Align.LEFT
  }

  private var slices: List<Slice> = emptyList()
  private var legendRows: List<RectF> = emptyList()
  private var cx = 0f
  private var cy = 0f
  private var outerRadius = 0f
  private var innerRadius = 0f
  private var legendX = 0f
  private var rowHeight = 0f

  override fun computePlot(w: Float, h: Float) {
    // Leave room on the right for the native legend.
    plot.set(dp(16f), dp(16f), w - dp(152f), h - dp(16f))
  }

  override fun drawChart(canvas: Canvas, spec: ChartSpec, area: RectF) {
    val entries = spec.entries
    if (entries.isEmpty() || spec.total <= 0.0 || area.width() <= 0 || area.height() <= 0) return

    cx = area.centerX()
    cy = area.centerY()
    outerRadius = min(area.width(), area.height()) / 2f - dp(10f)
    innerRadius = outerRadius * 0.56f
    if (outerRadius <= dp(8f)) return

    val outerOval = RectF(cx - outerRadius, cy - outerRadius, cx + outerRadius, cy + outerRadius)
    val innerOval = RectF(cx - innerRadius, cy - innerRadius, cx + innerRadius, cy + innerRadius)

    val builtSlices = ArrayList<Slice>(entries.size)
    var start = -90f // 12 o'clock; Canvas arc angles run clockwise on screen
    entries.forEachIndexed { index, entry ->
      val sweep = (entry.value / spec.total * 360f).toFloat()
      val path = Path()
      path.addArc(outerOval, start, sweep)
      path.arcTo(innerOval, start + sweep, -sweep, false)
      path.close()

      slicePaint.color = entry.color
      canvas.drawPath(path, slicePaint)
      canvas.drawPath(path, separationPaint)
      if (index == selectedIndex) canvas.drawPath(path, selectedSlicePaint)
      builtSlices.add(Slice(start, sweep))
      start += sweep
    }
    slices = builtSlices

    // Centre summary: total by default, selected datum when one is active.
    if (selectedIndex in entries.indices) {
      val entry = entries[selectedIndex]
      canvas.drawText("SELECTED", cx, cy - dp(18f), centerCaptionPaint)
      canvas.drawText(entry.label, cx, cy + dp(2f), centerCaptionPaint)
      canvas.drawText(ChartCatalog.format(entry.value), cx, cy + dp(24f), centerValuePaint)
    } else {
      canvas.drawText("TOTAL", cx, cy - dp(8f), centerCaptionPaint)
      canvas.drawText(ChartCatalog.format(spec.total), cx, cy + dp(18f), centerValuePaint)
    }

    // Native legend with tappable rows.
    legendX = area.right + dp(14f)
    rowHeight = min(dp(46f), area.height() / entries.size - dp(4f)).coerceAtLeast(dp(30f))
    val firstRowY = cy - (entries.size * rowHeight) / 2f
    val newRows = ArrayList<RectF>(entries.size)
    entries.forEachIndexed { index, entry ->
      val rowTop = firstRowY + index * rowHeight
      val row = RectF(legendX - dp(8f), rowTop, width.toFloat() - dp(4f), rowTop + rowHeight)
      newRows.add(row)
      if (index == selectedIndex) {
        canvas.drawRoundRect(row, dp(6f), dp(6f), rowHighlightPaint)
      }

      val swatchTop = rowTop + (rowHeight - dp(14f)) / 2f
      val swatch = RectF(legendX, swatchTop, legendX + dp(14f), swatchTop + dp(14f))
      swatchPaint.color = entry.color
      canvas.drawRoundRect(swatch, dp(3f), dp(3f), swatchPaint)

      val textX = legendX + dp(22f)
      val labelY = rowTop + rowHeight / 2f - dp(2f)
      canvas.drawText(entry.label, textX, labelY, legendLabelPaint)
      canvas.drawText(
        "${ChartCatalog.format(entry.value)}${spec.unit}",
        textX, labelY + dp(15f), legendValuePaint
      )
    }
    legendRows = newRows
  }

  override fun hitTest(x: Float, y: Float): Hit? {
    // Legend rows first — they are the easiest touch target.
    for (index in legendRows.indices) {
      val row = legendRows[index]
      if (x >= row.left && x <= row.right && y >= row.top && y <= row.bottom) {
        return Hit(index, legendX + dp(52f), row.centerY())
      }
    }

    // Donut sector.
    val dx = x - cx
    val dy = y - cy
    val distance = kotlin.math.sqrt(dx * dx + dy * dy)
    if (distance < innerRadius || distance > outerRadius) return null

    var angle = ((Math.toDegrees(atan2(dy.toDouble(), dx.toDouble())).toFloat() + 360f) % 360f)
    for (index in slices.indices) {
      val slice = slices[index]
      val startNorm = ((slice.start % 360f) + 360f) % 360f
      if (angle < startNorm) angle += 360f
      if (angle <= startNorm + slice.sweep) {
        val mid = Math.toRadians((slice.start + slice.sweep / 2f).toDouble())
        val anchorRadius = (innerRadius + outerRadius) / 2f
        return Hit(
          index,
          cx + anchorRadius * cos(mid).toFloat(),
          cy + anchorRadius * sin(mid).toFloat()
        )
      }
    }
    return null
  }
}
