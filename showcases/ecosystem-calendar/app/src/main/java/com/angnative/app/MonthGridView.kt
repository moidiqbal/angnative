/**
 * MonthGridView.kt — genuine native month-view calendar.
 *
 * This is a single custom android.view.View. Every pixel of the 7-column month
 * grid (weekday header, day numbers, current-day / selected-day highlights,
 * event dots, grid lines) is painted with android.graphics.Canvas + Paint in
 * onDraw(). Day selection is native touch hit-testing in onTouchEvent().
 *
 * There is no WebView, no HTML table, no DOM and no JavaScript anywhere in
 * this class. FullCalendar is a DOM/JS library and cannot run unchanged on a
 * native renderer; this is a native equivalent — see CalendarEvent.kt.
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
import java.util.GregorianCalendar
import kotlin.math.min

class MonthGridView @JvmOverloads constructor(
  context: Context,
  attrs: AttributeSet? = null
) : View(context, attrs) {

  /** A concrete grid date (month is 0-based, matching java.util.Calendar). */
  data class CellDate(val year: Int, val month: Int, val day: Int)

  /** Raised on tap, month navigation, or goToToday(). */
  var onDateSelected: ((CellDate) -> Unit)? = null

  private val today: CellDate
  private var year: Int
  private var month: Int
  private var selected: CellDate

  private val pad: Float
  private val headerH: Float
  private val numTextSize: Float
  private val weekCount = 6 // fixed 6-row grid, like FullCalendar's month view

  private val bgPaint = Paint(Paint.ANTI_ALIAS_FLAG)
  private val linePaint = Paint(Paint.ANTI_ALIAS_FLAG)
  private val weekdayPaint = Paint(Paint.ANTI_ALIAS_FLAG)
  private val numberPaint = Paint(Paint.ANTI_ALIAS_FLAG)
  private val adjacentPaint = Paint(Paint.ANTI_ALIAS_FLAG)
  private val todayBgPaint = Paint(Paint.ANTI_ALIAS_FLAG)
  private val todayTextPaint = Paint(Paint.ANTI_ALIAS_FLAG)
  private val selectedPaint = Paint(Paint.ANTI_ALIAS_FLAG)
  private val selectedTextPaint = Paint(Paint.ANTI_ALIAS_FLAG)
  private val todayRingPaint = Paint(Paint.ANTI_ALIAS_FLAG)
  private val dotPaint = Paint(Paint.ANTI_ALIAS_FLAG)
  private val selectedDotPaint = Paint(Paint.ANTI_ALIAS_FLAG)

  private val density: Float get() = resources.displayMetrics.density

  init {
    val now = GregorianCalendar()
    today = CellDate(
      now.get(GregorianCalendar.YEAR),
      now.get(GregorianCalendar.MONTH),
      now.get(GregorianCalendar.DAY_OF_MONTH)
    )
    year = today.year
    month = today.month
    selected = today

    val d = density
    pad = 10f * d
    headerH = 30f * d
    numTextSize = 15f * d

    bgPaint.color = Color.WHITE

    linePaint.color = Color.parseColor("#EEF2F7")
    linePaint.strokeWidth = d

    weekdayPaint.color = Color.parseColor("#64748B")
    weekdayPaint.textAlign = Paint.Align.CENTER
    weekdayPaint.textSize = 12f * d
    weekdayPaint.typeface = Typeface.DEFAULT_BOLD

    numberPaint.color = Color.parseColor("#0F172A")
    numberPaint.textAlign = Paint.Align.CENTER
    numberPaint.textSize = numTextSize
    numberPaint.typeface = Typeface.DEFAULT_BOLD

    adjacentPaint.color = Color.parseColor("#CBD5E1")
    adjacentPaint.textAlign = Paint.Align.CENTER
    adjacentPaint.textSize = numTextSize

    todayBgPaint.color = Color.parseColor("#DBEAFE")
    todayTextPaint.color = Color.parseColor("#1D4ED8")
    todayTextPaint.textAlign = Paint.Align.CENTER
    todayTextPaint.textSize = numTextSize
    todayTextPaint.typeface = Typeface.DEFAULT_BOLD

    selectedPaint.color = Color.parseColor("#4338CA")
    selectedTextPaint.color = Color.WHITE
    selectedTextPaint.textAlign = Paint.Align.CENTER
    selectedTextPaint.textSize = numTextSize
    selectedTextPaint.typeface = Typeface.DEFAULT_BOLD

    todayRingPaint.style = Paint.Style.STROKE
    todayRingPaint.color = Color.parseColor("#3B82F6")
    todayRingPaint.strokeWidth = 2f * d

    dotPaint.color = Color.parseColor("#6366F1")
    selectedDotPaint.color = Color.parseColor("#C7D2FE")
  }

  private fun colWidth(): Float = (width - 2f * pad) / 7f
  private fun cellHeight(): Float = (height - 2f * pad - headerH) / weekCount

  override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
    val w = MeasureSpec.getSize(widthMeasureSpec)
    val desired = (pad * 2f + headerH + weekCount * 54f * density).toInt()
    val h = when (MeasureSpec.getMode(heightMeasureSpec)) {
      MeasureSpec.EXACTLY -> MeasureSpec.getSize(heightMeasureSpec)
      MeasureSpec.AT_MOST -> min(desired, MeasureSpec.getSize(heightMeasureSpec))
      else -> desired
    }
    setMeasuredDimension(w, h)
  }

  override fun onDraw(canvas: Canvas) {
    super.onDraw(canvas)
    val w = width.toFloat()
    val h = height.toFloat()
    val colW = colWidth()
    val cellH = cellHeight()
    if (colW <= 0f || cellH <= 0f) return

    // Card background.
    canvas.drawRoundRect(RectF(0f, 0f, w, h), 14f * density, 14f * density, bgPaint)

    // Weekday header row (native text, not an HTML table).
    for (i in CalendarData.WEEKDAYS.indices) {
      val cx = pad + colW * (i + 0.5f)
      canvas.drawText(
        CalendarData.WEEKDAYS[i],
        cx,
        pad + headerH * 0.70f,
        weekdayPaint
      )
    }
    canvas.drawLine(pad, pad + headerH, w - pad, pad + headerH, linePaint)

    // Light grid rules.
    for (r in 1 until weekCount) {
      val y = pad + headerH + r * cellH
      canvas.drawLine(pad, y, w - pad, y, linePaint)
    }
    for (c in 1 until 7) {
      val x = pad + c * colW
      canvas.drawLine(x, pad + headerH, x, h - pad, linePaint)
    }

    // Walk all 42 cells from the Sunday on/before the 1st of the month.
    val first = GregorianCalendar(year, month, 1)
    val offset = first.get(GregorianCalendar.DAY_OF_WEEK) - first.firstDayOfWeek
    val cursor = GregorianCalendar(year, month, 1 - offset)
    val radius = min(colW, cellH) * 0.34f

    for (i in 0 until weekCount * 7) {
      val row = i / 7
      val col = i % 7
      val cellX = pad + col * colW
      val cellY = pad + headerH + row * cellH
      val cx = cellX + colW / 2f

      val cellYear = cursor.get(GregorianCalendar.YEAR)
      val cellMonth = cursor.get(GregorianCalendar.MONTH)
      val cellDay = cursor.get(GregorianCalendar.DAY_OF_MONTH)
      val inMonth = cellYear == year && cellMonth == month
      val isSelected = selected.year == cellYear &&
        selected.month == cellMonth && selected.day == cellDay
      val isToday = today.year == cellYear &&
        today.month == cellMonth && today.day == cellDay

      val numCy = cellY + cellH * 0.46f
      val textBaseline = numCy + numTextSize * 0.35f

      when {
        isSelected -> {
          canvas.drawCircle(cx, numCy, radius, selectedPaint)
          if (isToday) {
            canvas.drawCircle(cx, numCy, radius + 2f * density, todayRingPaint)
          }
        }
        isToday -> {
          val r = radius * 1.25f
          canvas.drawRoundRect(
            RectF(cx - r, numCy - r, cx + r, numCy + r),
            6f * density, 6f * density, todayBgPaint
          )
        }
      }

      val textPaint = when {
        isSelected -> selectedTextPaint
        isToday -> todayTextPaint
        !inMonth -> adjacentPaint
        else -> numberPaint
      }
      canvas.drawText(cellDay.toString(), cx, textBaseline, textPaint)

      // Deterministic event dots: up to three per day.
      val eventCount = CalendarData.eventsOn(cellYear, cellMonth, cellDay).size
      if (eventCount > 0) {
        val dots = min(3, eventCount)
        val gap = 9f * density
        val dotRadius = 2.6f * density
        val totalWidth = (dots - 1) * gap + 2f * dotRadius
        var dx = cx - totalWidth / 2f + dotRadius
        val dy = cellY + cellH * 0.86f
        val paint = if (isSelected) selectedDotPaint else dotPaint
        for (k in 0 until dots) {
          canvas.drawCircle(dx, dy, dotRadius, paint)
          dx += gap
        }
      }

      cursor.add(GregorianCalendar.DAY_OF_MONTH, 1)
    }
  }

  override fun onTouchEvent(event: MotionEvent): Boolean {
    when (event.actionMasked) {
      MotionEvent.ACTION_DOWN -> return true
      MotionEvent.ACTION_UP -> {
        val colW = colWidth()
        val cellH = cellHeight()
        val col = ((event.x - pad) / colW).toInt()
        val row = ((event.y - pad - headerH) / cellH).toInt()
        if (col in 0..6 && row in 0 until weekCount) {
          val first = GregorianCalendar(year, month, 1)
          val offset = first.get(GregorianCalendar.DAY_OF_WEEK) - first.firstDayOfWeek
          val cursor = GregorianCalendar(year, month, 1 - offset)
          cursor.add(GregorianCalendar.DAY_OF_MONTH, row * 7 + col)
          val picked = CellDate(
            cursor.get(GregorianCalendar.YEAR),
            cursor.get(GregorianCalendar.MONTH),
            cursor.get(GregorianCalendar.DAY_OF_MONTH)
          )
          selected = picked
          if (picked.year != year || picked.month != month) {
            year = picked.year
            month = picked.month
          }
          invalidate()
          onDateSelected?.invoke(picked)
          performClick()
          return true
        }
      }
    }
    return super.onTouchEvent(event)
  }

  override fun performClick(): Boolean {
    super.performClick()
    return true
  }

  /** Move the displayed month by +/-1, keeping a valid selected day. */
  fun shiftMonth(delta: Int) {
    val c = GregorianCalendar(year, month, 1)
    c.add(GregorianCalendar.MONTH, delta)
    year = c.get(GregorianCalendar.YEAR)
    month = c.get(GregorianCalendar.MONTH)
    val clampedDay = minOf(selected.day, c.getActualMaximum(GregorianCalendar.DAY_OF_MONTH))
    selected = CellDate(year, month, clampedDay)
    invalidate()
    onDateSelected?.invoke(selected)
  }

  fun goToToday() {
    year = today.year
    month = today.month
    selected = today
    invalidate()
    onDateSelected?.invoke(selected)
  }

  fun selectedDate(): CellDate = selected
  fun displayedTitle(): String = CalendarData.monthTitle(year, month)
}
