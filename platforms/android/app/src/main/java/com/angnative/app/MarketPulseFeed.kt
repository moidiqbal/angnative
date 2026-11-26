/**
 * MarketPulseFeed.kt — socket-only BTC/USD market data for the MarketPulse demo.
 *
 * A minimal RFC 6455 client connects to the Coinbase Exchange `matches` channel
 * over TLS (SSLSocket; no new Gradle dependency, minSdk 24). Every `match`
 * message is one executed trade: its `price`, `size` and `time` are the ONLY
 * inputs used to build 30-minute OHLC bars and real base-BTC volume. No REST,
 * no seeded history, no fabricated ticks, and no rolling/derived volume are
 * used anywhere in this file.
 *
 * UI pushes are coalesced on the main looper to at most ~4/sec, with a final
 * flush on stop or failure. On failure the last-good bars are retained and the
 * payload carries `fresh: false`, `streamState: "reconnecting"` and an error;
 * the socket then reconnects with exponential backoff.
 */
package com.angnative.app

import android.os.Handler
import android.os.Looper
import android.util.Base64
import android.util.Log
import org.json.JSONArray
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.io.IOException
import java.io.InputStream
import java.io.OutputStream
import java.net.InetSocketAddress
import java.net.Socket
import java.security.MessageDigest
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.Random
import java.util.TimeZone
import java.util.TreeMap
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit
import javax.net.ssl.SSLSocket
import javax.net.ssl.SSLSocketFactory
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToLong

object MarketPulseFeed {
  private const val TAG = "AngNative"
  private const val HOST = "ws-feed.exchange.coinbase.com"
  private const val PORT = 443
  private const val PRODUCT = "BTC-USD"
  private const val SYMBOL = "BTC/USD"
  private const val FEED =
    "Coinbase Exchange WebSocket · matches · 30m bars from executed matches · volume from match sizes"
  // Coinbase Exchange currently accepts channel names as a string array; the
  // object form (`[{"name":"matches"}]`) replies "is not a valid product" and
  // closes the socket. Verified with a live Node WebSocket subscription.
  private const val SUBSCRIBE_JSON =
    "{\"type\":\"subscribe\",\"product_ids\":[\"BTC-USD\"],\"channels\":[\"matches\"]}"
  private const val WS_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"

  private const val BUCKET_MS = 1_800_000L
  private const val MAX_BARS = 60
  private const val MAX_FRAME_BYTES = 1024 * 1024
  private const val MAX_HEADER_BYTES = 8192
  private const val CONNECT_TIMEOUT_MS = 10_000
  private const val READ_TIMEOUT_MS = 15_000
  private const val THROTTLE_MS = 250L
  private const val BACKOFF_START_MS = 1_000L
  private const val BACKOFF_MAX_MS = 30_000L

  private val worker = Executors.newSingleThreadExecutor { runnable ->
    Thread(runnable, "market-pulse-websocket").apply { isDaemon = true }
  }
  private val mainHandler = Handler(Looper.getMainLooper())
  private val publishLock = Any()
  private val random = Random()

  private val isoFormat = SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'", Locale.US).apply {
    timeZone = TimeZone.getTimeZone("UTC")
  }

  /** bucket-start epoch millis -> OHLC + accumulated real match size (BTC). */
  private val bars = TreeMap<Long, Bar>()

  @Volatile
  private var latestPayload: String? = null
  @Volatile
  private var lastFetchedAt = ""
  @Volatile
  private var stopped = true
  @Volatile
  private var stopSignal: CountDownLatch? = null
  @Volatile
  private var activeSocket: Socket? = null
  @Volatile
  private var onQuote: ((String) -> Unit)? = null

  private var pendingPayload: String? = null
  private var flushPosted = false

  private data class Bar(
    var o: Double,
    var h: Double,
    var l: Double,
    var c: Double,
    var volumeBtc: Double,
  )

  /** Honest pre-stream payload: no bars, no price, no error. */
  fun initialPayload(): String = JSONObject()
    .put("symbol", SYMBOL)
    .put("feed", FEED)
    .put("fetchedAt", "")
    .put("fresh", false)
    .put("streamState", "connecting")
    .put("bars", JSONArray())
    .toString()

  /** Last payload emitted to the UI (initial payload until the first event). */
  fun latestPayload(): String = latestPayload ?: initialPayload()

  /** Start the TLS WebSocket stream; [onQuote] is always called on the main thread. */
  @Synchronized
  fun startStreaming(onQuote: (String) -> Unit) {
    stopInternal(emitFinal = false)
    stopped = false
    val latch = CountDownLatch(1)
    stopSignal = latch
    this.onQuote = onQuote
    val connecting = buildPayload(fresh = false, state = "connecting", error = null)
    latestPayload = connecting
    publish(connecting)
    worker.execute { runStream(latch) }
  }

  /** Stop the stream and flush any coalesced quote that is still pending. */
  fun stopStreaming() = stopInternal(emitFinal = true)

  /** Compatibility alias for older tooling that used the host poller. */
  fun stopPolling() = stopStreaming()

  @Synchronized
  private fun stopInternal(emitFinal: Boolean) {
    stopped = true
    stopSignal?.countDown()
    try {
      activeSocket?.close()
    } catch (_: IOException) {
      // Already closed by the worker; nothing to do.
    }
    activeSocket = null
    val callback = onQuote
    onQuote = null
    if (emitFinal && callback != null) {
      mainHandler.post { flushPending(callback) }
    }
  }

  /* =======================================================================
   * Connection / reconnection loop (worker thread only)
   * ======================================================================= */

  private fun runStream(latch: CountDownLatch) {
    var backoffMs = BACKOFF_START_MS
    var failureEmitted = false
    while (!stopped) {
      var sawMatch = false
      try {
        connectAndStream { sawMatch = true }
      } catch (e: InterruptedException) {
        Thread.currentThread().interrupt()
        break
      } catch (e: Exception) {
        if (stopped) break
        if (!failureEmitted) {
          val reason = "${e.javaClass.simpleName}: ${e.message ?: "no message"}"
          Log.w(TAG, "MarketPulse WebSocket unavailable: $reason")
          latestPayload = buildPayload(fresh = false, state = "reconnecting", error = reason)
          publish(latestPayload!!)
          failureEmitted = true
        }
      }
      if (stopped) break
      if (sawMatch) {
        // A successful connection resets the backoff and clears the error flag.
        backoffMs = BACKOFF_START_MS
        failureEmitted = false
      } else {
        backoffMs = min(BACKOFF_MAX_MS, backoffMs * 2)
      }
      if (!awaitStop(latch, backoffMs)) break
    }
  }

  private fun awaitStop(latch: CountDownLatch, delayMs: Long): Boolean {
    return try {
      !latch.await(delayMs, TimeUnit.MILLISECONDS)
    } catch (e: InterruptedException) {
      Thread.currentThread().interrupt()
      false
    }
  }

  private fun connectAndStream(onMatch: () -> Unit) {
    val plain = Socket()
    var ssl: SSLSocket? = null
    activeSocket = plain
    try {
      plain.connect(InetSocketAddress(HOST, PORT), CONNECT_TIMEOUT_MS)
      val factory = SSLSocketFactory.getDefault() as? SSLSocketFactory
        ?: throw IOException("TLS is unavailable")
      val secure = factory.createSocket(plain, HOST, PORT, true) as SSLSocket
      ssl = secure
      activeSocket = secure
      secure.soTimeout = READ_TIMEOUT_MS
      secure.startHandshake()

      val input = secure.getInputStream()
      val output = secure.getOutputStream()
      val keyBytes = ByteArray(16).also { random.nextBytes(it) }
      val key = Base64.encodeToString(keyBytes, Base64.NO_WRAP)
      writeHandshake(output, key)
      validateHandshake(readHandshakeResponse(input), key)

      writeFrame(output, OPCODE_TEXT, SUBSCRIBE_JSON.toByteArray(Charsets.UTF_8))
      latestPayload = buildPayload(fresh = false, state = "live", error = null)
      publish(latestPayload!!)

      readFrames(input, output, onMatch)
    } finally {
      try {
        (ssl ?: plain).close()
      } catch (_: IOException) {
        // Ignore close failures; reconnection will open a fresh socket.
      }
      if (activeSocket === plain || activeSocket === ssl || activeSocket == null) {
        activeSocket = null
      }
    }
  }

  private fun writeHandshake(output: OutputStream, key: String) {
    val request = buildString {
      append("GET / HTTP/1.1\r\n")
      append("Host: $HOST\r\n")
      append("Upgrade: websocket\r\n")
      append("Connection: Upgrade\r\n")
      append("Sec-WebSocket-Key: $key\r\n")
      append("Sec-WebSocket-Version: 13\r\n")
      append("User-Agent: angnative-marketpulse/1\r\n")
      append("\r\n")
    }
    output.write(request.toByteArray(Charsets.US_ASCII))
    output.flush()
  }

  private fun readHandshakeResponse(input: InputStream): String {
    val bytes = ByteArray(1)
    val buffer = ByteArrayOutputStream()
    while (buffer.size() < MAX_HEADER_BYTES) {
      if (input.read(bytes, 0, 1) != 1) throw IOException("handshake closed before headers")
      buffer.write(bytes[0].toInt())
      val text = buffer.toString("ISO-8859-1")
      if (text.endsWith("\r\n\r\n")) return text
    }
    throw IOException("handshake headers exceeded $MAX_HEADER_BYTES bytes")
  }

  private fun validateHandshake(response: String, key: String) {
    val statusLine = response.substringBefore("\r\n")
    if (!statusLine.contains(" 101 ")) {
      throw IOException("WebSocket upgrade failed: $statusLine")
    }
    val lower = response.lowercase(Locale.US)
    if (!lower.contains("upgrade: websocket") || !lower.contains("connection: upgrade")) {
      throw IOException("WebSocket upgrade headers missing")
    }
    val accept = response.lineSequence()
      .firstOrNull { it.lowercase(Locale.US).startsWith("sec-websocket-accept:") }
      ?.substringAfter(':')
      ?.trim()
    val digest = MessageDigest.getInstance("SHA-1").digest((key + WS_GUID).toByteArray(Charsets.US_ASCII))
    val expected = Base64.encodeToString(digest, Base64.NO_WRAP)
    if (accept == null || !accept.equals(expected, ignoreCase = true)) {
      throw IOException("WebSocket accept verification failed")
    }
  }

  /* =======================================================================
   * Minimal RFC 6455 frame client
   * ======================================================================= */

  private fun writeFrame(output: OutputStream, opcode: Int, payload: ByteArray) {
    val length = payload.size
    val headerSize = when {
      length <= 125 -> 2
      length <= 0xFFFF -> 4
      else -> 10
    }
    val frame = ByteArray(headerSize + 4 + length)
    frame[0] = (0x80 or opcode).toByte()
    when {
      length <= 125 -> frame[1] = (0x80 or length).toByte()
      length <= 0xFFFF -> {
        frame[1] = (0x80 or 126).toByte()
        frame[2] = (length ushr 8).toByte()
        frame[3] = length.toByte()
      }
      else -> {
        frame[1] = (0x80 or 127).toByte()
        for (i in 0 until 8) frame[2 + i] = (length.toLong() ushr (56 - 8 * i)).toByte()
      }
    }
    val mask = ByteArray(4).also { random.nextBytes(it) }
    System.arraycopy(mask, 0, frame, headerSize, 4)
    for (i in payload.indices) {
      frame[headerSize + 4 + i] = (payload[i].toInt() xor mask[i % 4].toInt()).toByte()
    }
    output.write(frame)
    output.flush()
  }

  private fun readFrames(input: InputStream, output: OutputStream, onMatch: () -> Unit) {
    var fragment: ByteArrayOutputStream? = null
    while (!stopped) {
      val first = input.read()
      if (first < 0) throw IOException("socket closed during frame header")
      val second = input.read()
      if (second < 0) throw IOException("socket closed during frame header")

      val fin = (first and 0x80) != 0
      val opcode = first and 0x0F
      val masked = (second and 0x80) != 0
      var length = (second and 0x7F).toLong()
      when (length.toInt()) {
        126 -> {
          val ext = readExactly(input, 2)
          length = ((ext[0].toInt() and 0xFF).toLong() shl 8) or (ext[1].toInt() and 0xFF).toLong()
        }
        127 -> {
          val ext = readExactly(input, 8)
          length = 0L
          for (byte in ext) length = (length shl 8) or (byte.toInt() and 0xFF).toLong()
        }
      }
      if (length > MAX_FRAME_BYTES) throw IOException("frame too large: $length bytes")
      if (opcode >= 0x8 && length > 125) throw IOException("oversized control frame")

      val mask = if (masked) readExactly(input, 4) else null
      val payload = readExactly(input, length.toInt())
      if (mask != null) {
        for (i in payload.indices) {
          payload[i] = (payload[i].toInt() xor mask[i % 4].toInt()).toByte()
        }
      }

      when (opcode) {
        OPCODE_TEXT -> {
          if (fin) {
            handleText(String(payload, Charsets.UTF_8), onMatch)
          } else {
            val buffer = ByteArrayOutputStream()
            buffer.write(payload, 0, payload.size)
            fragment = buffer
          }
        }
        OPCODE_CONTINUATION -> {
          val buffer = fragment ?: throw IOException("continuation frame without a text start")
          if (buffer.size() + payload.size > MAX_FRAME_BYTES) {
            throw IOException("fragmented frame too large")
          }
          buffer.write(payload, 0, payload.size)
          if (fin) {
            handleText(buffer.toString("UTF-8"), onMatch)
            fragment = null
          }
        }
        OPCODE_PING -> writeFrame(output, OPCODE_PONG, payload)
        OPCODE_PONG -> Unit // keep-alive acknowledgement; nothing to do
        OPCODE_CLOSE -> {
          writeFrame(output, OPCODE_CLOSE, ByteArray(0))
          throw IOException("server sent close frame")
        }
        else -> Unit // binary frames and reserved opcodes are not used here
      }
    }
  }

  private fun readExactly(input: InputStream, size: Int): ByteArray {
    val out = ByteArray(size)
    var offset = 0
    while (offset < size) {
      val read = input.read(out, offset, size - offset)
      if (read < 0) throw IOException("socket closed mid-frame")
      offset += read
    }
    return out
  }

  private const val OPCODE_CONTINUATION = 0x0
  private const val OPCODE_TEXT = 0x1
  private const val OPCODE_CLOSE = 0x8
  private const val OPCODE_PING = 0x9
  private const val OPCODE_PONG = 0xA

  /* =======================================================================
   * Match-derived bars — executed matches only, never fabricated
   * ======================================================================= */

  private fun handleText(text: String, onMatch: () -> Unit) {
    val message = try {
      JSONObject(text)
    } catch (_: Exception) {
      return
    }
    when (message.optString("type")) {
      "match" -> applyExecutedMatch(message, "size", onMatch)
      // Defensive fallback only: the subscription is for `matches`, not ticker.
      "ticker" -> applyExecutedMatch(message, "last_size", onMatch)
      else -> Unit // subscriptions, heartbeats and snapshots carry no trade
    }
  }

  private fun applyExecutedMatch(message: JSONObject, sizeKey: String, onMatch: () -> Unit) {
    if (message.optString("product_id") != PRODUCT) return
    val price = finite(message, "price") ?: return
    if (price <= 0.0) return
    val size = max(0.0, finite(message, sizeKey) ?: 0.0)

    val epochMillis = parseEpochMillis(message.optString("time"))
    val bucket = Math.floorDiv(epochMillis, BUCKET_MS) * BUCKET_MS
    val bar = bars[bucket]
    if (bar == null) {
      bars[bucket] = Bar(price, price, price, price, size)
    } else {
      bar.h = max(bar.h, price)
      bar.l = min(bar.l, price)
      bar.c = price
      bar.volumeBtc += size
    }
    while (bars.size > MAX_BARS) bars.pollFirstEntry()

    lastFetchedAt = formatIso(System.currentTimeMillis())
    onMatch()
    latestPayload = buildPayload(fresh = true, state = "live", error = null)
    publish(latestPayload!!)
  }

  private fun finite(obj: JSONObject, key: String): Double? {
    val text = obj.optString(key)
    val value = text.toDoubleOrNull() ?: obj.optDouble(key, Double.NaN)
    return if (value.isNaN() || value.isInfinite()) null else value
  }

  private fun parseEpochMillis(text: String?): Long {
    if (text.isNullOrBlank()) return System.currentTimeMillis()
    var normalized = text.trim()
    val dot = normalized.indexOf('.')
    if (dot >= 0) {
      val end = normalized.indexOf('Z', dot)
      val fractionEnd = if (end >= 0) end else normalized.length
      val digits = normalized.substring(dot + 1, fractionEnd)
      val millis = (digits + "000").take(3)
      normalized = normalized.substring(0, dot + 1) + millis + normalized.substring(fractionEnd)
    } else if (normalized.endsWith("Z")) {
      // Coinbase normally sends fractional seconds; accept whole seconds too.
      normalized = normalized.dropLast(1) + ".000Z"
    }
    return try {
      isoFormat.parse(normalized)?.time ?: System.currentTimeMillis()
    } catch (_: Exception) {
      System.currentTimeMillis()
    }
  }

  private fun formatIso(epochMillis: Long): String =
    synchronized(isoFormat) { isoFormat.format(Date(epochMillis)) }

  private fun buildPayload(fresh: Boolean, state: String, error: String?): String {
    val array = JSONArray()
    for ((bucket, bar) in bars) {
      array.put(
        JSONObject()
          .put("t", bucket)
          .put("o", bar.o)
          .put("h", bar.h)
          .put("l", bar.l)
          .put("c", bar.c)
          // Milli-BTC integer from real match sizes; never hardcoded or derived
          // from a rolling 24h volume field.
          .put("v", max(0L, (bar.volumeBtc * 1000.0).roundToLong())),
      )
    }
    val payload = JSONObject()
      .put("symbol", SYMBOL)
      .put("feed", FEED)
      .put("fetchedAt", lastFetchedAt)
      .put("fresh", fresh)
      .put("streamState", state)
      .put("bars", array)
    if (error != null) payload.put("error", error)
    return payload.toString()
  }

  /* =======================================================================
   * Throttled main-thread delivery (~4/sec) with a final coalesced flush
   * ======================================================================= */

  private fun publish(payload: String) {
    synchronized(publishLock) {
      pendingPayload = payload
      if (!flushPosted) {
        flushPosted = true
        mainHandler.postDelayed({ flushPending(null) }, THROTTLE_MS)
      }
    }
  }

  private fun flushPending(callback: ((String) -> Unit)?) {
    val payload = synchronized(publishLock) {
      flushPosted = false
      val pending = pendingPayload
      pendingPayload = null
      pending
    }
    if (payload != null) (callback ?: onQuote)?.invoke(payload)
  }
}
