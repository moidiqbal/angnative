/**
 * NativeUiTheme.kt — overridable default colors/sizes for the stock widgets.
 * Consuming showcases can supply a copy with their own palette without having
 * to provide a full [NativeViewCreator].
 */
package com.angnative.ui.core

data class NativeUiTheme(
  val textColor: Int = StyleCodec.parseColor("#0F172A"),
  val textSizeSp: Float = StyleCodec.DEFAULT_TEXT_SIZE_SP,
  val inputTextColor: Int = StyleCodec.BLACK,
  val buttonBackground: Int = StyleCodec.parseColor("#2563EB"),
  val buttonTextColor: Int = StyleCodec.WHITE,
  val buttonTextSizeSp: Float = 16f,
  val buttonCornerRadiusDp: Float = 12f,
  val buttonAllCaps: Boolean = false
) {
  companion object {
    @JvmField
    val DEFAULT: NativeUiTheme = NativeUiTheme()
  }
}
