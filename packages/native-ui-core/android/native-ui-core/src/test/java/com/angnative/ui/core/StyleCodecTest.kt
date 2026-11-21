package com.angnative.ui.core

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Pure-JVM tests: the style parsing and view-type mapping are Android-free by
 * design, so these run with plain JUnit — no emulator or Robolectric needed.
 * Actual View instantiation is covered by the showcase apps' build/UI checks.
 */
class StyleCodecTest {

  @Test
  fun dimensionUnitsResolveAgainstDeviceMetrics() {
    assertEquals(24, StyleCodec.parseDimension("12dp").toPixels(2f, 1000))
    assertEquals(12, StyleCodec.parseDimension("12px").toPixels(2f, 1000))
    assertEquals(250, StyleCodec.parseDimension("25%").toPixels(2f, 1000))
    assertEquals(7, StyleCodec.parseDimension("7").toPixels(2f, 1000))
    assertEquals(StyleCodec.WRAP_CONTENT, StyleCodec.parseDimension("auto").toPixels(2f, 1000))
  }

  @Test
  fun malformedDimensionsFallBackToZero() {
    assertEquals(0, StyleCodec.parseDimension("nonsense").toPixels(2f, 1000))
    assertEquals(0, StyleCodec.parseDimension("px").toPixels(2f, 1000))
  }

  @Test
  fun colorsParseHexAndNamedValues() {
    assertEquals(StyleCodec.parseColor("#2563EB"), StyleCodec.parseColor("#2563eb"))
    assertEquals(0xFF2563EB.toInt(), StyleCodec.parseColor("#2563EB"))
    assertEquals(0x80334455.toInt(), StyleCodec.parseColor("#80334455"))
    assertEquals(0xFFFF00FF.toInt(), StyleCodec.parseColor("#F0F"))
    assertEquals(StyleCodec.WHITE, StyleCodec.parseColor("white"))
    assertEquals(StyleCodec.TRANSPARENT, StyleCodec.parseColor("transparent"))
    assertEquals(StyleCodec.BLACK, StyleCodec.parseColor("#not-a-color"))
  }

  @Test
  fun textSizeSpecPixelsMapToSp() {
    assertEquals(11f, StyleCodec.parseTextSize("11px", 2f, 1.5f), 0f)
    assertEquals(20f, StyleCodec.parseTextSize("10dp", 2f, 1f), 0f)
    assertEquals(15f, StyleCodec.parseTextSize("bad", 2f, 1f), 0f)
  }

  @Test
  fun fontWeightOpacityAndVisibility() {
    assertEquals(StyleCodec.TYPEFACE_BOLD, StyleCodec.parseFontWeight("bold"))
    assertEquals(StyleCodec.TYPEFACE_BOLD, StyleCodec.parseFontWeight("700"))
    assertEquals(StyleCodec.TYPEFACE_NORMAL, StyleCodec.parseFontWeight("400"))
    assertEquals(1f, StyleCodec.parseOpacity("junk"), 0f)
    assertEquals(0.5f, StyleCodec.parseOpacity("0.5"), 0f)
    assertEquals(StyleCodec.GONE, StyleCodec.parseVisibility("hidden"))
    assertEquals(StyleCodec.VISIBLE, StyleCodec.parseVisibility("visible"))
  }

  @Test
  fun viewTypeMappingCoversAllRustNodeTypes() {
    assertEquals(NativeViewKind.CONTAINER, ViewTypes.kindFor("View"))
    assertEquals(NativeViewKind.CONTAINER, ViewTypes.kindFor("Custom"))
    assertEquals(NativeViewKind.CONTAINER, ViewTypes.kindFor("ScrollView"))
    assertEquals(NativeViewKind.TEXT, ViewTypes.kindFor("Text"))
    assertEquals(NativeViewKind.TEXT_INPUT, ViewTypes.kindFor("TextInput"))
    assertEquals(NativeViewKind.BUTTON, ViewTypes.kindFor("Button"))
    assertEquals(NativeViewKind.IMAGE, ViewTypes.kindFor("Image"))
    assertEquals(NativeViewKind.SWITCH, ViewTypes.kindFor("Switch"))
    assertEquals(NativeViewKind.ACTIVITY_INDICATOR, ViewTypes.kindFor("ActivityIndicator"))
    assertEquals(NativeViewKind.PLAIN, ViewTypes.kindFor("SomethingNew"))
  }
}
