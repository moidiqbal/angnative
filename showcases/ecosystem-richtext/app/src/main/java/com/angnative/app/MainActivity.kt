/**
 * MainActivity.kt — Entry point for the AngNative native rich-text showcase.
 *
 * The Rust JNI bridge is kept intact and is deliberately exercised at startup:
 * nativeHostCreate logs "AngNative host created" (logcat tag "AngNative") and a
 * command batch is dispatched through the C ABI with its response logged here.
 * The visible editor itself is 100% native Android (EditText + Spannable spans) —
 * no WebView, no DOM, no HTML rendering.
 */

package com.angnative.app

import android.os.Bundle
import android.util.Log
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.WindowCompat

class MainActivity : AppCompatActivity() {

  companion object {
    private const val TAG = "AngNativeRichText"
  }

  private lateinit var bridge: JniBridge

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)

    WindowCompat.setDecorFitsSystemWindows(window, true)
    supportActionBar?.hide()

    // Loads libangnative.so, built from jni_bridge.cpp + prebuilt Rust FFI libs.
    AngNativeApp.initialize(applicationContext)

    // Exercise the Rust JNI bridge at startup (host creation + batch response).
    // jni_bridge.cpp emits: "AngNative host created: 0x..." under tag "AngNative".
    bridge = JniBridge(this)
    bridge.initHost()

    val batch = buildBatch {
      createNode(1, "View")
      setStyle(1, "width", "100%")
      setText(1, "richtext-bridge-bootstrap")
    }
    val response = bridge.processBatch(batch)
    Log.i(TAG, "Startup JNI batch response: $response")
    Log.i(TAG, "Rust shadow-tree node count: ${bridge.nodeCount()}")

    // All visible content is native Android; the bridge is alive but not used
    // to render the editor (Tiptap/CKEditor cannot run unchanged natively).
    setContentView(RichTextEditorView(this, bridge.nodeCount()))
  }

  override fun onDestroy() {
    bridge.destroyHost()
    super.onDestroy()
  }
}
