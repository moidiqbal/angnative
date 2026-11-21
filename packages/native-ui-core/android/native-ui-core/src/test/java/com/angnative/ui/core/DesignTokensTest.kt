package com.angnative.ui.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/** Pure-JVM tests — no emulator needed. */
class DesignTokensTest {

  @Test
  fun all_five_presets_are_distinct() {
    val colors = DesignTokens.ALL.map { it.second.buttonBackground }
    assertEquals(5, colors.toSet().size)
  }

  @Test
  fun all_presets_have_valid_text_and_button_colors() {
    for ((name, theme) in DesignTokens.ALL) {
      assertTrue("$name text color is opaque", (theme.textColor.toLong() and 0xFF000000L) == 0xFF000000L)
      assertEquals("$name button text", StyleCodec.WHITE, theme.buttonTextColor)
      assertNotEquals("$name button bg vs text", theme.buttonTextColor, theme.buttonBackground)
    }
  }

  @Test
  fun presets_use_positive_sizes() {
    for ((name, theme) in DesignTokens.ALL) {
      assertTrue("$name text size", theme.textSizeSp in 12f..24f)
      assertTrue("$name button size", theme.buttonTextSizeSp in 12f..20f)
      assertTrue("$name corner radius", theme.buttonCornerRadiusDp in 0f..32f)
    }
  }
}
