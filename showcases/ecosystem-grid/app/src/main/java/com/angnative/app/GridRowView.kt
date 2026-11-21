/**
 * GridRowView.kt — builds the column header row and each recycled row view.
 *
 * The exact same column weights are used for the header and for every row so
 * the header stays aligned with the data while scrolling. Row height is fixed
 * (on purpose): RecyclerView.setHasFixedSize(true) is only valid when every
 * item has the same height, and that is what makes the scroll smooth over
 * 10,000 rows.
 *
 * NATIVE EQUIVALENT of an AG Grid column layout — see Employee.kt for why
 * AG Grid itself (DOM-only) cannot run here.
 */
package com.angnative.app

import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.widget.LinearLayout
import android.widget.TextView

object GridRowView {

  // Generated IDs are unique process-wide; safe for findViewById in a ViewHolder.
  val ID_ID = View.generateViewId()
  val ID_NAME = View.generateViewId()
  val ID_DEPT = View.generateViewId()
  val ID_SALARY = View.generateViewId()

  const val HEADER_HEIGHT_DP = 44
  const val ROW_HEIGHT_DP = 56

  /** Column weights. Shared by header + rows => guaranteed alignment. */
  private const val ID_WIDTH_DP = 52
  private const val NAME_WEIGHT = 2.2f
  private const val DEPT_WEIGHT = 1.5f
  private const val SALARY_WEIGHT = 1.6f

  fun build(context: Context, header: Boolean): LinearLayout {
    val row = LinearLayout(context).apply {
      orientation = LinearLayout.HORIZONTAL
      gravity = Gravity.CENTER_VERTICAL
      setPadding(dp(context, 12), 0, dp(context, 12), 0)
      setBackgroundColor(
        if (header) Color.parseColor("#E6EDF6") else Color.WHITE
      )
    }

    row.addView(
      cell(
        context, header, "ID", ID_ID,
        LinearLayout.LayoutParams(dp(context, ID_WIDTH_DP), LinearLayout.LayoutParams.MATCH_PARENT),
        Gravity.CENTER
      )
    )
    row.addView(
      cell(
        context, header, "Employee", ID_NAME,
        LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, NAME_WEIGHT),
        Gravity.CENTER_VERTICAL or Gravity.START
      )
    )
    row.addView(
      cell(
        context, header, "Department", ID_DEPT,
        LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, DEPT_WEIGHT),
        Gravity.CENTER_VERTICAL or Gravity.START
      )
    )
    row.addView(
      cell(
        context, header, "Salary", ID_SALARY,
        LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, SALARY_WEIGHT),
        Gravity.CENTER_VERTICAL or Gravity.END
      )
    )
    return row
  }

  private fun cell(
    context: Context,
    header: Boolean,
    text: String,
    id: Int,
    params: LinearLayout.LayoutParams,
    gravity: Int
  ): TextView = TextView(context).apply {
    this.id = id
    layoutParams = params
    this.gravity = gravity
    this.text = text
    typeface = if (header) Typeface.DEFAULT_BOLD else Typeface.DEFAULT
    setTextSize(TypedValue.COMPLEX_UNIT_SP, if (header) 12f else 14f)
    setTextColor(
      if (header) Color.parseColor("#334155")
      else Color.parseColor("#0F172A")
    )
    // Single line keeps the row height stable — critical for virtualization.
    maxLines = 1
    isSingleLine = true
    ellipsize = android.text.TextUtils.TruncateAt.END
  }

  private fun dp(context: Context, value: Int): Int =
    (value * context.resources.displayMetrics.density).toInt()
}
