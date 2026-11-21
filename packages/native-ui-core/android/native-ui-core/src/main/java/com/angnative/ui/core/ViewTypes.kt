/**
 * ViewTypes.kt — the pure mapping from a Rust `view_type` string to the kind of
 * Android widget the factory should create. Kept Android-free so the mapping
 * can be unit tested; NativeViewFactory turns each kind into a real View.
 */
package com.angnative.ui.core

enum class NativeViewKind {
  CONTAINER,
  TEXT,
  TEXT_INPUT,
  BUTTON,
  IMAGE,
  SWITCH,
  ACTIVITY_INDICATOR,
  PLAIN
}

object ViewTypes {
  /** Map a PlatformOp::Create view_type. Unknown types become a plain View. */
  fun kindFor(viewType: String): NativeViewKind = when (viewType.trim()) {
    "View", "Custom", "ScrollView" -> NativeViewKind.CONTAINER
    "Text" -> NativeViewKind.TEXT
    "TextInput" -> NativeViewKind.TEXT_INPUT
    "Button" -> NativeViewKind.BUTTON
    "Image" -> NativeViewKind.IMAGE
    "Switch" -> NativeViewKind.SWITCH
    "ActivityIndicator" -> NativeViewKind.ACTIVITY_INDICATOR
    else -> NativeViewKind.PLAIN
  }
}
