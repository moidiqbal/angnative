/**
 * MainActivity.kt — Entry point for the AngNative Android shell.
 *
 * Hosts the AngNativeView which renders the native view tree via JNI
 * to the Rust host (angnative-host-ffi C ABI).
 */

package com.angnative.app

import android.os.Bundle
import android.view.Window
import android.view.WindowManager
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.WindowCompat

class MainActivity : AppCompatActivity() {

  private lateinit var angNativeView: AngNativeView

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)

    WindowCompat.setDecorFitsSystemWindows(window, true)
    supportActionBar?.hide()

    // Initialize the AngNative native host
    AngNativeApp.initialize(applicationContext)

    // Create the native view that bridges to the Rust host
    angNativeView = AngNativeView(this)
    setContentView(angNativeView)

    // MarketPulse fetches the public BTC/USD OHLC payload on a worker thread first.
    // The QuickJS fetch shim is synchronous, so the host caches the normalized
    // payload and registers it before the Angular AOT bundle boots. Builds
    // without the MarketPulse asset keep the original counter boot path.
    if (angNativeView.hasMarketPulseBundle()) {
      MarketPulseFeed.fetch { quote ->
        runOnUiThread { angNativeView.loadAngularApp(quote) }
      }
    } else {
      angNativeView.loadAngularApp()
    }
  }

  override fun onResume() {
    super.onResume()
    angNativeView.onResume()
  }

  override fun onPause() {
    super.onPause()
    angNativeView.onPause()
  }

  override fun onDestroy() {
    super.onDestroy()
    angNativeView.destroy()
  }
}
