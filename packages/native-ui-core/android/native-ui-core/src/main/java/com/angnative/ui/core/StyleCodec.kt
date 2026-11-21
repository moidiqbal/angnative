/**
 * StyleCodec.kt — pure JVM parsing helpers used by [NativeViewFactory].
 *
 * This file intentionally contains no `android.*` imports. The integer values
 * below mirror the framework constants (Color / Typeface / View /
 * ViewGroup.LayoutParams), so the parsing rules can be verified by ordinary
 * local JVM unit tests without Robolectric or an emulator. NativeViewFactory
 * feeds these values straight into the Android APIs.
 */
package com.angnative.ui.core

object StyleCodec {
  // Framework-compatible constants (android.view.View / Typeface / LayoutParams).
  const val WRAP_CONTENT: Int = -2
  const val MATCH_PARENT: Int = -1
  const val TYPEFACE_NORMAL: Int = 0
  const val TYPEFACE_BOLD: Int = 1
  const val VISIBLE: Int = 0
  const val INVISIBLE: Int = 4
  const val GONE: Int = 8

  // Framework-compatible ARGB colors (android.graphics.Color).
  const val BLACK: Int = 0xFF000000.toInt()
  const val WHITE: Int = 0xFFFFFFFF.toInt()
  const val RED: Int = 0xFFFF0000.toInt()
  const val GREEN: Int = 0xFF00FF00.toInt()
  const val BLUE: Int = 0xFF0000FF.toInt()
  const val GRAY: Int = 0xFF888888.toInt()
  const val TRANSPARENT: Int = 0x00000000

  const val DEFAULT_TEXT_SIZE_SP: Float = 15f

  enum class DimensionUnit { PX, DP, PERCENT, AUTO, RAW }

  /** A parsed dimension. Unresolved units are resolved by [toPixels] against device metrics. */
  data class Dimension(val value: Float, val unit: DimensionUnit) {
    /**
     * Resolve to device pixels.
     *
     * `auto` becomes WRAP_CONTENT. `RAW` numbers are already device pixels.
     * Percentages resolve against [widthPixels] (the Rust style table has no
     * per-axis percentage distinction), matching the historical factory code.
     */
    fun toPixels(density: Float, widthPixels: Int): Int = when (unit) {
      DimensionUnit.PX, DimensionUnit.RAW -> value.toInt()
      DimensionUnit.DP -> (value * density).toInt()
      DimensionUnit.PERCENT -> (value / 100f * widthPixels).toInt()
      DimensionUnit.AUTO -> WRAP_CONTENT
    }
  }

  /** Parse CSS-like dimension strings: "12px", "12dp", "25%", "auto", or a bare number. */
  fun parseDimension(raw: String): Dimension {
    val value = raw.trim()
    return when {
      value.equals("auto", ignoreCase = true) -> Dimension(0f, DimensionUnit.AUTO)
      value.endsWith("px", ignoreCase = true) ->
        Dimension(value.dropLast(2).toFloatOrNull() ?: 0f, DimensionUnit.PX)
      value.endsWith("dp", ignoreCase = true) ->
        Dimension(value.dropLast(2).toFloatOrNull() ?: 0f, DimensionUnit.DP)
      value.endsWith("%") ->
        Dimension(value.dropLast(1).toFloatOrNull() ?: 0f, DimensionUnit.PERCENT)
      else -> Dimension(value.toFloatOrNull() ?: 0f, DimensionUnit.RAW)
    }
  }

  /**
   * Parse font size. Logical/spec px map 1:1 to sp; dp values convert through
   * density and fontScale (the old divide-by-scaledDensity produced tiny text).
   */
  fun parseTextSize(
    raw: String,
    density: Float,
    fontScale: Float,
    defaultSp: Float = DEFAULT_TEXT_SIZE_SP
  ): Float {
    val value = raw.trim()
    val scale = if (fontScale > 0f) fontScale else 1f
    return when {
      value.endsWith("px", ignoreCase = true) -> value.dropLast(2).toFloatOrNull() ?: defaultSp
      value.endsWith("dp", ignoreCase = true) ->
        (value.dropLast(2).toFloatOrNull() ?: defaultSp) * density / scale
      else -> value.toFloatOrNull() ?: defaultSp
    }
  }

  /**
   * Parse #RGB, #ARGB, #RRGGBB, #AARRGGBB and the small set of CSS named colors
   * emitted by the style table. Unknown/malformed values return BLACK.
   */
  fun parseColor(raw: String): Int {
    val value = raw.trim()
    if (value.startsWith("#")) {
      val hex = value.substring(1)

      fun nibble(index: Int): Int? = hex.getOrNull(index)?.let { c ->
        when (c) {
          in '0'..'9' -> c - '0'
          in 'a'..'f' -> c - 'a' + 10
          in 'A'..'F' -> c - 'A' + 10
          else -> null
        }
      }

      fun byteAt(index: Int): Int? {
        val hi = nibble(index) ?: return null
        val lo = nibble(index + 1) ?: return null
        return (hi shl 4) or lo
      }

      return when (hex.length) {
        3 -> {
          val r = nibble(0) ?: return BLACK
          val g = nibble(1) ?: return BLACK
          val b = nibble(2) ?: return BLACK
          argb(255, r * 17, g * 17, b * 17)
        }
        4 -> {
          val a = nibble(0) ?: return BLACK
          val r = nibble(1) ?: return BLACK
          val g = nibble(2) ?: return BLACK
          val b = nibble(3) ?: return BLACK
          argb(a * 17, r * 17, g * 17, b * 17)
        }
        6 -> {
          val r = byteAt(0) ?: return BLACK
          val g = byteAt(2) ?: return BLACK
          val b = byteAt(4) ?: return BLACK
          argb(255, r, g, b)
        }
        8 -> {
          val a = byteAt(0) ?: return BLACK
          val r = byteAt(2) ?: return BLACK
          val g = byteAt(4) ?: return BLACK
          val b = byteAt(6) ?: return BLACK
          argb(a, r, g, b)
        }
        else -> BLACK
      }
    }
    return when (value.lowercase()) {
      "white" -> WHITE
      "black" -> BLACK
      "red" -> RED
      "green" -> GREEN
      "blue" -> BLUE
      "gray", "grey" -> GRAY
      "transparent" -> TRANSPARENT
      else -> BLACK
    }
  }

  /** Typeface style integers: bold/700+ map to BOLD, everything else to NORMAL. */
  fun parseFontWeight(raw: String): Int = when (raw.trim().lowercase()) {
    "bold", "700", "800", "900" -> TYPEFACE_BOLD
    else -> TYPEFACE_NORMAL
  }

  /** Opacity in [0, 1]; malformed input falls back to fully opaque. */
  fun parseOpacity(raw: String): Float =
    raw.trim().toFloatOrNull()?.coerceIn(0f, 1f) ?: 1f

  /** Only "hidden"/"gone" hide the view; every other value is VISIBLE. */
  fun parseVisibility(raw: String): Int = when (raw.trim().lowercase()) {
    "hidden", "gone" -> GONE
    else -> VISIBLE
  }

  private fun argb(a: Int, r: Int, g: Int, b: Int): Int =
    (a shl 24) or (r shl 16) or (g shl 8) or b
}
