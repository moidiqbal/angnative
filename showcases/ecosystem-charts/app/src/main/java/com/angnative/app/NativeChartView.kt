/**
 * NativeChartView.kt — shared base for the native Canvas charts.
 *
 * Everything here is pure android.graphics: no WebView, no DOM, no chart
 * library. Subclasses implement [drawChart] with Canvas.draw* calls and
 * [hitTest] so a tap can open the value tooltip drawn by this base class.
 */

package com.angnative.app

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.Typeface
import android.util.AttributeSet
import android.view.MotionEvent
import android.view.View

abstract class NativeChartView @JvmOverloads constructor(
  context: Context,
  attrs: AttributeSet? = null
) : View(context, attrs) {

  protected val density: Float = resources.displayMetrics.density
  protected fun dp(value: Float): Float = value * density

  protected val plot = RectF()
  protected var selectedIndex: Int = -1
    private set

  private var spec: ChartSpec? = null
  private var anchorX = 0f
  private var anchorY = 0f

  protected val gridPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    color = Color.parseColor("#E2E8F0")
    style = Paint.Style.STROKE
    strokeWidth = dp(1f)
  }
  protected val axisPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    color = Color.parseColor("#CBD5E1")
    style = Paint.Style.STROKE
    strokeWidth = dp(1.5f)
  }
  protected val yLabelPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    color = Color.parseColor("#94A3B8")
    textSize = dp(11f)
    textAlign = Paint.Align.RIGHT
  }
  protected val xLabelPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    color = Color.parseColor("#64748B")
    textSize = dp(11f)
    textAlign = Paint.Align.CENTER
  }
  protected val valuePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    color = Color.parseColor("#334155")
    textSize = dp(11f)
    textAlign = Paint.Align.CENTER
    typeface = Typeface.DEFAULT_BOLD
  }
  private val tooltipBgPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    color = Color.parseColor("#1E293B")
  }
  private val tooltipTextPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
    color = Color.WHITE
    textSize = dp(12f)
    textAlign = Paint.Align.LEFT
    typeface = Typeface.DEFAULT_BOLD
  }

  /** The point where the tooltip should point; set by [hitTest] results. */
  protected data class Hit(val index: Int, val x: Float, val y: Float)

  fun setSpec(value: ChartSpec) {
    spec = value
    selectedIndex = -1
    invalidate()
  }

  final override fun onDraw(canvas: Canvas) {
    super.onDraw(canvas)
    val current = spec ?: return
    if (width <= 0 || height <= 0) return
    computePlot(width.toFloat(), height.toFloat())
    drawChart(canvas, current, plot)
    if (selectedIndex in current.entries.indices) {
      drawTooltip(canvas, current.entries[selectedIndex], current)
    }
  }

  /** Reserve space for axis labels. Pie/donut overrides this for its legend. */
  protected open fun computePlot(w: Float, h: Float) {
    plot.set(dp(52f), dp(18f), w - dp(18f), h - dp(42f))
  }

  /** Horizontal gridlines plus value labels on the y axis. */
  protected fun drawGrid(canvas: Canvas, area: RectF, max: Double, steps: Int = 4) {
    for (i in 0..steps) {
      val y = area.bottom - area.height() * i / steps
      canvas.drawLine(area.left, y, area.right, y, if (i == 0) axisPaint else gridPaint)
      val label = ChartCatalog.format(max * i / steps)
      canvas.drawText(label, area.left - dp(8f), y + dp(4f), yLabelPaint)
    }
  }

  protected abstract fun drawChart(canvas: Canvas, spec: ChartSpec, area: RectF)

  /** Return the tapped datum and the tooltip anchor, or null when empty space was tapped. */
  protected abstract fun hitTest(x: Float, y: Float): Hit?

  override fun onTouchEvent(event: MotionEvent): Boolean {
    when (event.actionMasked) {
      MotionEvent.ACTION_DOWN -> return true
      MotionEvent.ACTION_UP -> {
        val hit = hitTest(event.x, event.y)
        selectedIndex = hit?.index ?: -1
        if (hit != null) {
          anchorX = hit.x
          anchorY = hit.y
        }
        invalidate()
        performClick()
        return true
      }
      MotionEvent.ACTION_CANCEL -> return true
    }
    return super.onTouchEvent(event)
  }

  // Supplying a click action keeps TalkBack/a11y linters happy for touch Views.
  override fun performClick(): Boolean {
    super.performClick()
    return true
  }

  private fun drawTooltip(canvas: Canvas, entry: ChartEntry, spec: ChartSpec) {
    val text = "${entry.label}  ${ChartCatalog.format(entry.value)} ${spec.unit}"
    val pad = dp(10f)
    val boxH = dp(38f)
    val boxW = tooltipTextPaint.measureText(text) + pad * 2
    val margin = dp(4f)

    var left = anchorX - boxW / 2f
    var top = anchorY - boxH - dp(14f)
    if (left < margin) left = margin
    if (left + boxW > width - margin) left = width - boxW - margin
    if (top < margin) top = anchorY + dp(14f)
    if (top + boxH > height - margin) top = height - boxH - margin

    val box = RectF(left, top, left + boxW, top + boxH)
    canvas.drawRoundRect(box, dp(8f), dp(8f), tooltipBgPaint)
    val fm = tooltipTextPaint.fontMetrics
    val baseline = box.centerY() - (fm.ascent + fm.descent) / 2f
    canvas.drawText(text, box.left + pad, baseline, tooltipTextPaint)
  }
}
