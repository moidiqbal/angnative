/**
 * EmployeeAdapter.kt — genuine Android RecyclerView virtualization.
 *
 * Only ~10-15 ViewHolder instances exist at any time no matter how many of the
 * 10,000 rows are on screen. When a row scrolls out of the viewport Android
 * calls onViewRecycled(), the ViewHolder is placed in the scrap/recycled pool,
 * and onBindViewHolder() rebinds it with new data. That is the same mechanism
 * an AG Grid virtual viewport uses in the DOM, implemented here with NATIVE
 * views — no WebView, no DOM, no JS.
 */
package com.angnative.app

import android.graphics.Color
import android.graphics.Typeface
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import androidx.recyclerview.widget.RecyclerView
import java.util.Locale

class EmployeeAdapter(
  private val employees: List<Employee>,
  private val onRecycled: (Int) -> Unit = {}
) : RecyclerView.Adapter<EmployeeAdapter.EmployeeViewHolder>() {

  /** Monotonic counter used by the header badge to make recycling visible. */
  private var recycledCount = 0

  init {
    // Stable IDs let RecyclerView reuse ViewHolders deterministically on
    // notifyDataSetChanged()/scroll — equivalent to AG Grid's getRowId().
    setHasStableIds(true)
  }

  override fun getItemCount(): Int = employees.size

  override fun getItemId(position: Int): Long = employees[position].id

  override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): EmployeeViewHolder {
    val row = GridRowView.build(parent.context, header = false)
    row.layoutParams = RecyclerView.LayoutParams(
      RecyclerView.LayoutParams.MATCH_PARENT,
      dp(parent.context, GridRowView.ROW_HEIGHT_DP)
    )
    return EmployeeViewHolder(row)
  }

  override fun onBindViewHolder(holder: EmployeeViewHolder, position: Int) {
    holder.bind(employees[position])
  }

  override fun onViewRecycled(holder: EmployeeViewHolder) {
    recycledCount++
    onRecycled(recycledCount)
    super.onViewRecycled(holder)
  }

  fun currentRecycleCount(): Int = recycledCount

  class EmployeeViewHolder(view: View) : RecyclerView.ViewHolder(view) {
    private val idView: TextView = view.findViewById(GridRowView.ID_ID)
    private val nameView: TextView = view.findViewById(GridRowView.ID_NAME)
    private val deptView: TextView = view.findViewById(GridRowView.ID_DEPT)
    private val salaryView: TextView = view.findViewById(GridRowView.ID_SALARY)

    fun bind(employee: Employee) {
      idView.text = employee.id.toString()
      nameView.text = employee.name
      deptView.text = employee.department
      salaryView.text = String.format(Locale.US, "\$%,d", employee.salary)
      salaryView.typeface = Typeface.DEFAULT_BOLD
      salaryView.setTextColor(Color.parseColor("#0F766E"))
      idView.setTextColor(Color.parseColor("#64748B"))
      // Zebra striping — computed from the data, no extra view hierarchy.
      itemView.setBackgroundColor(
        if (employee.id % 2L == 0L) Color.parseColor("#F6F8FC") else Color.WHITE
      )
    }
  }

  private fun dp(context: android.content.Context, value: Int): Int =
    (value * context.resources.displayMetrics.density).toInt()
}
