/**
 * MarketPulseFeed.kt — host-side fetch adapter for the crypto MarketPulse demo.
 *
 * Angular's QuickJS fetch shim is synchronous, so the network work happens
 * here on a background executor and the normalized payload is registered as
 * a host-cached response before the AOT bundle boots. Refresh uses the same
 * adapter and pushes fresh JSON into Angular through __marketPulseApplyQuote.
 *
 * The source is CoinGecko's public BTC/USD OHLC endpoint (30-minute bars for
 * `days=1`). It does not return volume, so `v` is explicitly 0. Delays, rate
 * limits and failures are expected; no data is fabricated on failure.
 */
package com.angnative.app

import android.util.Log
import org.json.JSONArray
import org.json.JSONObject
import java.io.BufferedReader
import java.io.InputStreamReader
import java.net.HttpURLConnection
import java.net.URL
import java.util.concurrent.Executors

object MarketPulseFeed {
  private const val TAG = "AngNative"
  private const val QUOTE_URL =
    "https://api.coingecko.com/api/v3/coins/bitcoin/ohlc?vs_currency=usd&days=1"
  private const val SYMBOL = "BTC/USD"
  private const val FEED = "CoinGecko public OHLC · 30m bars"
  private const val TIMEOUT_MS = 8000
  private const val MAX_BARS = 60

  private val executor = Executors.newSingleThreadExecutor { runnable ->
    Thread(runnable, "market-pulse-feed").apply { isDaemon = true }
  }

  @Volatile
  private var lastGood: String? = null

  /** Fetch on a worker thread. The callback runs on that worker; callers post to main. */
  fun fetch(onResult: (String) -> Unit) {
    executor.execute {
      val payload = try {
        val normalized = normalize(loadBars())
        lastGood = normalized
        normalized
      } catch (e: Exception) {
        val reason = "${e.javaClass.simpleName}: ${e.message ?: "no message"}"
        Log.w(TAG, "MarketPulse crypto feed unavailable: $reason")
        withError(lastGood, reason)
      }
      onResult(payload)
    }
  }

  private fun loadBars(): JSONArray {
    val connection = URL(QUOTE_URL).openConnection() as HttpURLConnection
    connection.connectTimeout = TIMEOUT_MS
    connection.readTimeout = TIMEOUT_MS
    connection.requestMethod = "GET"
    try {
      val code = connection.responseCode
      if (code != HttpURLConnection.HTTP_OK) throw IllegalStateException("HTTP $code")
      val text = BufferedReader(InputStreamReader(connection.inputStream, Charsets.UTF_8)).use {
        it.readText()
      }
      return JSONArray(text)
    } finally {
      connection.disconnect()
    }
  }

  /**
   * CoinGecko OHLC rows are [timestamp_ms, open, high, low, close].
   * Normalize them into the documented bars contract used by na-candlestick.
   */
  private fun normalize(rows: JSONArray): String {
    if (rows.length() == 0) throw IllegalArgumentException("empty OHLC response")

    val parsed = mutableListOf<JSONObject>()
    for (i in 0 until rows.length()) {
      val row = rows.optJSONArray(i)
        ?: throw IllegalArgumentException("OHLC row $i is not an array")
      if (row.length() < 5) throw IllegalArgumentException("OHLC row $i needs 5 values")
      val t = row.optLong(0)
      val open = row.optDouble(1)
      val high = row.optDouble(2)
      val low = row.optDouble(3)
      val close = row.optDouble(4)
      if (listOf(open, high, low, close).any { it.isNaN() || it.isInfinite() }) continue
      if (high < low) throw IllegalArgumentException("high < low at OHLC row $i")
      parsed.add(
        JSONObject()
          .put("t", t)
          .put("o", open)
          .put("h", high)
          .put("l", low)
          .put("c", close)
          .put("v", 0),
      )
    }
    if (parsed.isEmpty()) throw IllegalArgumentException("no finite OHLC bars")

    val bars = JSONArray()
    parsed.sortedBy { it.optLong("t") }.takeLast(MAX_BARS).forEach { bars.put(it) }
    return JSONObject()
      .put("symbol", SYMBOL)
      .put("feed", FEED)
      .put("fetchedAt", java.time.Instant.now().toString())
      .put("bars", bars)
      .toString()
  }

  /** Last good bars stay visible; the error is explicit in the payload. */
  private fun withError(cached: String?, reason: String): String {
    val base = cached?.let { runCatching { JSONObject(it) }.getOrNull() }
      ?: JSONObject()
        .put("symbol", SYMBOL)
        .put("feed", FEED)
        .put("fetchedAt", "")
        .put("bars", JSONArray())
    return base.put("error", reason).toString()
  }
}
