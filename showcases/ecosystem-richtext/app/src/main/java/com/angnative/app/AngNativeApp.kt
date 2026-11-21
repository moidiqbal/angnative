/**
 * AngNativeApp.kt — Application-level bootstrap for AngNative on Android.
 *
 * Provides the entry point for initializing the native host and loading
 * the demo application. Called from MainActivity.onCreate().
 */

package com.angnative.app

import android.content.Context
import android.widget.Toast

object AngNativeApp {

  private var isInitialized: Boolean = false

  /** Initialize the AngNative native host. */
  fun initialize(context: Context) {
    if (isInitialized) return

    try {
      // Verify the native library is available
      System.loadLibrary("angnative")
      isInitialized = true
    } catch (e: UnsatisfiedLinkError) {
      Toast.makeText(
        context,
        "AngNative native library not found. Build libangnative.so for Android first.",
        Toast.LENGTH_LONG
      ).show()
      throw e
    }
  }

  /** Check if AngNative is initialized. */
  fun isInitialized(): Boolean = isInitialized

  /** Shutdown the AngNative host. */
  fun shutdown() {
    isInitialized = false
  }
}
