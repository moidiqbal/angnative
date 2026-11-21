/**
 * Employee.kt — data model for the native virtualized grid showcase.
 *
 * HONESTY NOTE (AG Grid):
 * AG Grid (ag-grid-community / ag-grid-angular) is a DOM + CSS + JavaScript
 * library. It requires an HTML DOM, browser layout and a JS runtime, none of
 * which exist in AngNative's native Android renderer. It therefore CANNOT run
 * unchanged here. The data below is generated locally in Kotlin and rendered
 * by a real android RecyclerView with Adapter + ViewHolder recycling. This is
 * a NATIVE EQUIVALENT of an AG Grid data grid (10k rows, virtualization,
 * stable IDs, fixed row height) — NOT the AG Grid library itself.
 */
package com.angnative.app

import kotlin.random.Random

/** One row of the grid: mirrors the four fields of a typical AG Grid column def. */
data class Employee(
  val id: Long,
  val name: String,
  val department: String,
  val salary: Long
)

object EmployeeData {
  /** Number of rows in the grid — large enough to prove virtualization. */
  const val ROW_COUNT = 10_000

  private val FIRST_NAMES = arrayOf(
    "Ava", "Liam", "Mia", "Noah", "Zoe", "Ethan", "Isla", "Leo", "Nora", "Kai",
    "Maya", "Omar", "Priya", "Ravi", "Sofia", "Theo", "Uma", "Viktor", "Wren", "Yara"
  )
  private val LAST_NAMES = arrayOf(
    "Johnson", "Smith", "Patel", "Garcia", "Nguyen", "Okafor", "Rossi", "Kim",
    "Silva", "Muller", "Ivanov", "Haddad", "Novak", "Tanaka", "O'Connor", "Dubois",
    "Kowalski", "Fernandez", "Andersson", "Costa"
  )
  private val DEPARTMENTS = arrayOf(
    "Engineering", "Product", "Design", "Sales", "Marketing",
    "Finance", "Operations", "Support", "Research", "People"
  )

  /**
   * Deterministic generation (fixed seed) so screenshots and demos are
   * reproducible. Building 10,000 small objects on the main thread is <10 ms.
   */
  fun generate(count: Int = ROW_COUNT): List<Employee> {
    val random = Random(42L)
    val rows = ArrayList<Employee>(count)
    for (i in 1..count) {
      val first = FIRST_NAMES[random.nextInt(FIRST_NAMES.size)]
      val last = LAST_NAMES[random.nextInt(LAST_NAMES.size)]
      val dept = DEPARTMENTS[random.nextInt(DEPARTMENTS.size)]
      val salary = 45_000L + random.nextLong(0, 180_000L)
      rows.add(Employee(i.toLong(), "$first $last", dept, salary))
    }
    return rows
  }
}
