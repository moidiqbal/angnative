/**
 * DesignTokens.kt — real, testable token presets for native equivalents of
 * popular Angular design libraries.
 *
 * HONESTY: these are native Android theme values (colors/sizes) that imitate
 * each library's visual language. They are NOT the third-party libraries and do
 * not execute any of their code. Each preset maps to [NativeUiTheme], which the
 * shared [NativeViewFactory] already consumes.
 */
package com.angnative.ui.core

object DesignTokens {

  /** Material 3 inspired: indigo primary, rounded buttons. */
  @JvmField
  val MATERIAL_3 = NativeUiTheme(
    textColor = StyleCodec.parseColor("#1E293B"),
    textSizeSp = 16f,
    inputTextColor = StyleCodec.parseColor("#0F172A"),
    buttonBackground = StyleCodec.parseColor("#6750A4"),
    buttonTextColor = StyleCodec.WHITE,
    buttonTextSizeSp = 16f,
    buttonCornerRadiusDp = 20f,
    buttonAllCaps = false
  )

  /** PrimeNG inspired: blue primary, flatter corners. */
  @JvmField
  val PRIMENG = NativeUiTheme(
    textColor = StyleCodec.parseColor("#212529"),
    textSizeSp = 14f,
    inputTextColor = StyleCodec.parseColor("#212529"),
    buttonBackground = StyleCodec.parseColor("#0D6EFD"),
    buttonTextColor = StyleCodec.WHITE,
    buttonTextSizeSp = 14f,
    buttonCornerRadiusDp = 4f,
    buttonAllCaps = false
  )

  /** NG-ZORRO (Ant Design) inspired: blue primary, 6dp corners. */
  @JvmField
  val NG_ZORRO = NativeUiTheme(
    textColor = StyleCodec.parseColor("#262626"),
    textSizeSp = 14f,
    inputTextColor = StyleCodec.parseColor("#262626"),
    buttonBackground = StyleCodec.parseColor("#1890FF"),
    buttonTextColor = StyleCodec.WHITE,
    buttonTextSizeSp = 14f,
    buttonCornerRadiusDp = 6f,
    buttonAllCaps = false
  )

  /** Taiga UI inspired: violet primary, compact corners. */
  @JvmField
  val TAIGA = NativeUiTheme(
    textColor = StyleCodec.parseColor("#1B1B1F"),
    textSizeSp = 14f,
    inputTextColor = StyleCodec.parseColor("#1B1B1F"),
    buttonBackground = StyleCodec.parseColor("#7B61FF"),
    buttonTextColor = StyleCodec.WHITE,
    buttonTextSizeSp = 14f,
    buttonCornerRadiusDp = 8f,
    buttonAllCaps = false
  )

  /** Spartan UI inspired: near-black primary, sharp corners. */
  @JvmField
  val SPARTAN = NativeUiTheme(
    textColor = StyleCodec.parseColor("#0A0A0A"),
    textSizeSp = 14f,
    inputTextColor = StyleCodec.parseColor("#0A0A0A"),
    buttonBackground = StyleCodec.parseColor("#18181B"),
    buttonTextColor = StyleCodec.WHITE,
    buttonTextSizeSp = 14f,
    buttonCornerRadiusDp = 4f,
    buttonAllCaps = false
  )

  /** All presets, for iteration/tests. */
  @JvmField
  val ALL: List<Pair<String, NativeUiTheme>> = listOf(
    "material" to MATERIAL_3,
    "primeng" to PRIMENG,
    "ngzorro" to NG_ZORRO,
    "taiga" to TAIGA,
    "spartan" to SPARTAN
  )
}
