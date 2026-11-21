/**
 * CalendarEvent.kt — deterministic event seed for the native month calendar.
 *
 * HONESTY NOTE
 * ------------
 * FullCalendar is a DOM + JavaScript library (it renders into an HTML element
 * with a browser DOM and CSS). It cannot run unchanged on AngNative's native
 * renderer: this app has no DOM, no browser engine and zero WebView. The
 * events below and all calendar pixels are an explicitly NATIVE REIMPLEMENTATION
 * inspired by FullCalendar's month-view visual language, not unchanged
 * FullCalendar compatibility. No FullCalendar source or npm package is bundled.
 */

package com.angnative.app

import java.text.SimpleDateFormat
import java.util.GregorianCalendar
import java.util.Locale

/** One seeded event. [day] is a day-of-month (1..28). */
data class CalendarEvent(
  val day: Int,
  val time: String,
  val title: String,
  val category: String
)

object CalendarData {
  private const val SEED = 42L

  private val TITLES = listOf(
    "Sprint planning", "Design review", "Team standup", "1:1 with Alex",
    "Roadmap sync", "Code review", "Customer call", "Retrospective",
    "Focus block", "Release check", "Interview", "Docs workshop"
  )
  private val TIMES = listOf(
    "08:30", "09:00", "10:15", "11:00", "13:00", "14:30",
    "15:00", "16:15", "17:00"
  )
  private val CATEGORIES = listOf("Work", "Meeting", "Focus", "Personal", "Release")

  /**
   * Twelve deterministic events for the requested month. A small linear
   * congruential generator seeded from [year]/[month] picks distinct days
   * 1..28; the same month always yields the same twelve events.
   */
  fun eventsFor(year: Int, month: Int): List<CalendarEvent> {
    var state = SEED + year.toLong() * 131 + month.toLong() * 17
    val days = LinkedHashSet<Int>()
    while (days.size < 12) {
      state = (state * 1103515245L + 12345L) and 0x7FFFFFFF
      days.add(1 + (state % 28).toInt())
    }
    return days.sorted().mapIndexed { index, day ->
      CalendarEvent(
        day = day,
        time = TIMES[index % TIMES.size],
        title = TITLES[index % TITLES.size],
        category = CATEGORIES[index % CATEGORIES.size]
      )
    }
  }

  /** Events on one concrete date. */
  fun eventsOn(year: Int, month: Int, day: Int): List<CalendarEvent> =
    eventsFor(year, month).filter { it.day == day }

  /** e.g. "November 2026" */
  fun monthTitle(year: Int, month: Int): String =
    SimpleDateFormat("MMMM yyyy", Locale.US).format(GregorianCalendar(year, month, 1).time)

  /** e.g. "Sat, Nov 21, 2026" */
  fun fullDate(year: Int, month: Int, day: Int): String =
    SimpleDateFormat("EEE, MMM d, yyyy", Locale.US)
      .format(GregorianCalendar(year, month, day).time)

  /** Short weekday labels for the grid header, Sunday first. */
  val WEEKDAYS: List<String> =
    listOf("Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat")
}
