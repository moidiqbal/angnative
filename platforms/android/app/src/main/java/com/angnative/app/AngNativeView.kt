/**
 * AngNativeView.kt — Native container for the Angular-driven Android shell.
 *
 * The UI is not hardcoded in Kotlin. A real Angular AOT bundle is read from the
 * app assets, executed in the Rust-owned QuickJS runtime, and the resulting
 * command batches are materialized as Android views by NativeViewFactory.
 * Taps are dispatched back through the runtime bridge to live Angular listeners.
 * No WebView is involved.
 */

package com.angnative.app

import android.content.Context
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.os.Handler
import android.os.Looper
import android.util.AttributeSet
import android.util.Log
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import androidx.core.widget.NestedScrollView
import org.json.JSONObject

class AngNativeView(context: Context, attrs: AttributeSet? = null) :
  FrameLayout(context, attrs) {

  private val tag = "AngNative"
  private lateinit var jniBridge: JniBridge
  private var viewFactory: NativeViewFactory? = null
  private lateinit var statusView: TextView
  private val mainHandler = Handler(Looper.getMainLooper())
  private var destroyed = false
  private var latestQuote: String? = null
  private var marketPulsePolling = false
  private var layoutWidthDp = 0f
  private var layoutHeightDp = 600f

  companion object {
    /** Angular's root component node id, emitted by the AOT renderer. */
    private const val ROOT_NODE_ID = 1L
    /** Stable counter asset name asserted by the instrumentation suite. */
    private const val BUNDLE_ASSET = "angnative-bundle.js"
    /** MarketPulse demo asset, staged separately by the Gradle build. */
    private const val MARKET_PULSE_ASSET = "market-pulse-bundle.js"
  }

  init {
    jniBridge = JniBridge(context)
    jniBridge.initHost()

    layoutParams = LayoutParams(
      LayoutParams.MATCH_PARENT,
      LayoutParams.MATCH_PARENT
    )
    setBackgroundColor(Color.parseColor("#F8FAFC"))
  }

  /** Boot the compiled Angular app into the native view tree. */
  fun loadAngularApp(initialQuote: String? = null) {
    removeAllViews()

    val density = resources.displayMetrics.density
    fun toPx(dp: Int): Int = (dp * density).toInt()

    // Thin decorative shell. All visible app content comes from native ops
    // emitted by the Angular renderer inside the Rust runtime.
    val scrollView = NestedScrollView(context).apply {
      layoutParams = LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT)
      isFillViewport = true
    }
    val shell = LinearLayout(context).apply {
      orientation = LinearLayout.VERTICAL
      layoutParams = LinearLayout.LayoutParams(
        LinearLayout.LayoutParams.MATCH_PARENT,
        LinearLayout.LayoutParams.WRAP_CONTENT
      )
      setPadding(toPx(16), toPx(16), toPx(16), toPx(24))
    }
    statusView = TextView(context).apply {
      textSize = 13f
      setTextColor(Color.parseColor("#64748B"))
      setPadding(0, 0, 0, toPx(8))
    }
    val card = FrameLayout(context).apply {
      layoutParams = LinearLayout.LayoutParams(
        LinearLayout.LayoutParams.MATCH_PARENT,
        LinearLayout.LayoutParams.WRAP_CONTENT
      )
      background = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = 18f * density
        setColor(Color.WHITE)
        setStroke(toPx(1), Color.parseColor("#E2E8F0"))
      }
      elevation = 4f * density
      setPadding(toPx(20), toPx(22), toPx(20), toPx(22))
    }
    shell.addView(statusView)
    shell.addView(card)
    scrollView.addView(shell)
    addView(scrollView)

    var factory: NativeViewFactory? = null
    factory = NativeViewFactory(context, card) { nodeId ->
      dispatchClickWithAncestors(nodeId, factory)
    }
    viewFactory = factory
    jniBridge.viewFactory = factory
    factory.setRootNode(ROOT_NODE_ID)

    try {
      val bundleAsset =
        if (initialQuote != null || hasMarketPulseBundle()) MARKET_PULSE_ASSET else BUNDLE_ASSET
      initialQuote?.let { registerHostFetchHandler(it) }
      val bundle = readAngularBundle(bundleAsset)
      val rc = jniBridge.loadScript(bundle)
      if (rc != 0) {
        throw IllegalStateException("QuickJS rejected the Angular bundle (rc=$rc)")
      }

      // Layout is produced by the Rust host from the real shadow tree.
      layoutWidthDp = resources.displayMetrics.widthPixels / density
      val layoutJson = jniBridge.runLayout(layoutWidthDp, layoutHeightDp)
      val applied = factory.applyLayoutResult(layoutJson)
      if (!applied) {
        Log.w(tag, "Angular layout was not applied: $layoutJson; ${jniBridge.lastError()}")
      }

      val plusId = factory.findButtonIdByText("+")
      statusView.text = "Angular in Rust: nodes=${jniBridge.nodeCount()} rc=$rc plusNode=$plusId"
      card.tag = "angular-native-ui"
      plusId?.let { card.setTag(0x7F000001, it) }
      Log.i(tag, "Angular boot complete. nodes=${jniBridge.nodeCount()} plusNode=$plusId")
      if (bundleAsset == MARKET_PULSE_ASSET) startMarketPulsePolling()
    } catch (e: Exception) {
      statusView.text = "Angular bundle unavailable: ${e.message}"
      Log.e(tag, "Angular boot failed", e)
    }
  }

  /**
   * Angular may register (click) on a component host above the tappable view,
   * so walk the native ancestor chain until a listener actually responds.
   */
  private fun dispatchClickWithAncestors(nodeId: Long, factory: NativeViewFactory?) {
    var current: Long? = nodeId
    var hops = 0
    while (current != null && current != 0L && hops < 32) {
      val action = factory?.actionOf(current)
      val isRefresh = action == "refresh"
      if (isRefresh) refreshQuote()
      // na-pressable listens for `press`; keep `click` for existing (click) bindings.
      val clicked = jniBridge.dispatchNativeEvent(current, "click")
      val pressed = jniBridge.dispatchNativeEvent(current, "press")
      Log.i(
        tag,
        "Angular tap node=$current action=$action click=$clicked press=$pressed refresh=$isRefresh",
      )
      if (clicked > 0 || pressed > 0 || isRefresh) break
      current = factory?.parentOf(current)
      hops++
    }
  }

  /** Register the synchronous fetch handler Angular's HttpClient reads from. */
  private fun registerHostFetchHandler(quoteJson: String) {
    latestQuote = quoteJson
    val bootstrap = """
      globalThis.__marketPulseCachedQuote = JSON.parse(${JSONObject.quote(quoteJson)});
      __angnativeRegisterFetchHandler(function (req) {
        if (String(req.url).indexOf('/marketpulse/quote') >= 0) {
          return { status: 200, body: globalThis.__marketPulseCachedQuote };
        }
        return { status: 502, body: { error: 'unsupported fetch url' } };
      });
      0;
    """.trimIndent()
    val rc = jniBridge.loadScript(bootstrap)
    if (rc != 0) throw IllegalStateException("host fetch handler registration failed (rc=$rc)")
  }

  /**
   * Delayed host-side polling. The QuickJS `setInterval` shim is deliberately
   * one-shot, so a recurring JS timer would starve the bounded host job loop;
   * Kotlin owns the schedule and pushes into the Angular RxJS stream here.
   */
  private fun startMarketPulsePolling() {
    if (marketPulsePolling) return
    marketPulsePolling = true
    val intervalMs = System.getProperty("angnative.marketpulse.poll.ms")?.toLongOrNull()
      ?: MarketPulseFeed.DEFAULT_POLL_INTERVAL_MS
    MarketPulseFeed.startPolling(intervalMs) { quote ->
      mainHandler.post { applyHostQuote(quote, fromPoll = true) }
    }
  }

  /** Async host refetch; the result is pushed into Angular on the main looper. */
  private fun refreshQuote() {
    statusView.text = "refreshing BTC/USD OHLC..."
    MarketPulseFeed.fetch { quote ->
      mainHandler.post { applyHostQuote(quote) }
    }
  }

  private fun applyHostQuote(quoteJson: String, fromPoll: Boolean = false) {
    if (destroyed) return
    latestQuote = quoteJson
    val quoted = JSONObject.quote(quoteJson)
    val script = """
      globalThis.__marketPulseCachedQuote = JSON.parse($quoted);
      if (typeof __marketPulseApplyQuote === 'function') __marketPulseApplyQuote($quoted);
      0;
    """.trimIndent()
    val rc = jniBridge.loadScript(script)
    if (rc != 0) {
      Log.e(tag, "MarketPulse quote push failed (rc=$rc): ${jniBridge.lastError()}")
      statusView.text = "quote push failed (rc=$rc)"
      return
    }
    val layoutJson = jniBridge.runLayout(layoutWidthDp, layoutHeightDp)
    viewFactory?.applyLayoutResult(layoutJson)
    statusView.text = "Angular in Rust: nodes=${jniBridge.nodeCount()} · BTC/USD OHLC"
    if (fromPoll) {
      val bars = runCatching { JSONObject(quoteJson).optJSONArray("bars")?.length() ?: 0 }
        .getOrDefault(0)
      Log.i(tag, "MarketPulse poll pushed fresh quote: bars=$bars")
    }
  }

  /** True when the MarketPulse demo bundle was staged into app assets. */
  fun hasMarketPulseBundle(): Boolean =
    context.assets.list("")?.contains(MARKET_PULSE_ASSET) == true

  private fun readAngularBundle(asset: String): String = try {
    context.assets.open(asset).bufferedReader().use { it.readText() }
  } catch (e: Exception) {
    throw IllegalStateException(
      "Angular bundle '$asset' is missing; run the matching pnpm compile/bundle script",
      e
    )
  }

  /** Compatibility aliases used by older tooling/scripts. */
  fun loadDemoApp() {
    loadAngularApp()
  }

  @Deprecated("The shell now boots a compiled Angular bundle; use loadAngularApp().")
  fun loadHelloWorldApp() {
    loadAngularApp()
  }

  fun onResume() {}
  fun onPause() {}

  /** Last payload delivered by the host adapter (diagnostics/E2E assertions). */
  fun lastQuote(): String? = latestQuote

  fun destroy() {
    destroyed = true
    MarketPulseFeed.stopPolling()
    marketPulsePolling = false
    jniBridge.destroyHost()
    viewFactory?.clear()
    viewFactory = null
    removeAllViews()
  }
}
