/**
 * NativeViewSink.kt — receiving end of Rust-emitted ViewFactory PlatformOps.
 *
 * JniBridge exposes the eight JNI-resolvable `onNative*` methods; each one
 * forwards to the installed sink. Keeping the sink as an interface lets device
 * tests record callback thread/looper affinity without constructing real
 * Android views.
 */
package com.angnative.app

interface NativeViewSink {
  /** PlatformOp::Create */
  fun onNativeCreateView(nodeId: Long, viewType: String)

  /** PlatformOp::SetFrame */
  fun onNativeSetFrame(nodeId: Long, x: Float, y: Float, width: Float, height: Float)

  /** PlatformOp::SetProps */
  fun onNativeSetProps(nodeId: Long, propsJson: String)

  /** PlatformOp::SetText */
  fun onNativeSetText(nodeId: Long, text: String)

  /** PlatformOp::Append */
  fun onNativeAppendChild(parentId: Long, childId: Long)

  /** PlatformOp::Insert */
  fun onNativeInsertChild(parentId: Long, childId: Long, index: Int)

  /** PlatformOp::Remove */
  fun onNativeRemoveChild(parentId: Long, childId: Long)

  /** PlatformOp::Destroy */
  fun onNativeDestroyView(nodeId: Long)
}
