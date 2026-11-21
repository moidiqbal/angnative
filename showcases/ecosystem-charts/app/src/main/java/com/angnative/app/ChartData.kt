/**
 * ChartData.kt — data for the AngNative native chart showcase.
 *
 * HONESTY NOTE: Apache ECharts (and its Angular wrapper ng2-charts) is a
 * DOM / browser-canvas JavaScript library. It cannot execute unchanged on a
 * native renderer: there is no DOM, no browser <canvas> and (deliberately) no
 * WebView in this project. No ECharts source is bundled here. The numbers
 * below are generated in Kotlin and the visuals in BarChartView,
 * LineChartView and PieChartView are a NATIVE REIMPLEMENTATION of familiar
 * ECharts chart types drawn with android.graphics.Canvas.
 */

package com.angnative.app

import android.graphics.Color

/** One categorical value in a chart series. */
data class ChartEntry(
  val label: String,
  val value: Double,
  val color: Int
)

enum class ChartKind { BAR, LINE, PIE }

/** Everything a chart View needs: kind, header copy, unit and its data. */
data class ChartSpec(
  val kind: ChartKind,
  val title: String,
  val description: String,
  val unit: String,
  val entries: List<ChartEntry>
) {
  val total: Double get() = entries.sumOf { it.value }
  val max: Double get() = entries.maxOfOrNull { it.value } ?: 1.0
}

object ChartCatalog {

  /** Formatting shared by axes, tooltips and the pie centre label. */
  fun format(value: Double): String =
    if (value % 1.0 == 0.0) value.toLong().toString()
    else String.format(java.util.Locale.US, "%.1f", value)

  private fun c(hex: String): Int = Color.parseColor(hex)

  /** Data is generated here (Kotlin), not loaded from any chart library. */
  val charts: List<ChartSpec> = listOf(
    ChartSpec(
      kind = ChartKind.BAR,
      title = "Bar chart — Quarterly revenue",
      description = "Six quarters of revenue in USD millions. Drawn as native Canvas rounded bars with gridlines and axis labels.",
      unit = "M USD",
      entries = listOf(
        ChartEntry("Q1 '25", 12.4, c("#60A5FA")),
        ChartEntry("Q2 '25", 18.1, c("#3B82F6")),
        ChartEntry("Q3 '25", 15.6, c("#2563EB")),
        ChartEntry("Q4 '25", 24.3, c("#1D4ED8")),
        ChartEntry("Q1 '26", 28.9, c("#4F46E5")),
        ChartEntry("Q2 '26", 33.2, c("#7C3AED"))
      )
    ),
    ChartSpec(
      kind = ChartKind.LINE,
      title = "Line chart — Weekly active sessions",
      description = "A seven-day trend in thousands of sessions. The line, area fill and points are native Canvas Paths.",
      unit = "K sessions",
      entries = listOf(
        ChartEntry("Mon", 42.0, c("#0EA5E9")),
        ChartEntry("Tue", 55.0, c("#0EA5E9")),
        ChartEntry("Wed", 49.0, c("#0EA5E9")),
        ChartEntry("Thu", 71.0, c("#0EA5E9")),
        ChartEntry("Fri", 66.0, c("#0EA5E9")),
        ChartEntry("Sat", 84.0, c("#0EA5E9")),
        ChartEntry("Sun", 92.0, c("#0EA5E9"))
      )
    ),
    ChartSpec(
      kind = ChartKind.PIE,
      title = "Donut chart — Traffic sources",
      description = "Acquisition mix as a share of total visits. Slices are native Canvas arcs; tap a slice or legend row for its value.",
      unit = "%",
      entries = listOf(
        ChartEntry("Direct", 32.0, c("#6366F1")),
        ChartEntry("Organic", 28.0, c("#06B6D4")),
        ChartEntry("Referral", 18.0, c("#10B981")),
        ChartEntry("Social", 14.0, c("#F59E0B")),
        ChartEntry("Email", 8.0, c("#EC4899"))
      )
    )
  )
}
