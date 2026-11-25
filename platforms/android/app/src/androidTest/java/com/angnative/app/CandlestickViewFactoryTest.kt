package com.angnative.app

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.util.concurrent.atomic.AtomicReference

/**
 * Device-side proof that the `Custom:na-candlestick` wire tag materializes a
 * native CandlestickView and that the JSON-string OHLC payload is parsed.
 * No WebView or injected one-off view is involved.
 */
@RunWith(AndroidJUnit4::class)
class CandlestickViewFactoryTest {

    @Test
    fun taggedCandlestickCreatesNativeViewAndParsesBarsString() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val error = AtomicReference<Throwable>()

        instrumentation.runOnMainSync {
            try {
                val context = instrumentation.targetContext
                val container = android.widget.FrameLayout(context)
                val factory = NativeViewFactory(context, container)

                factory.onNativeCreateView(1L, NativeViewFactory.CUSTOM_CANDLESTICK_TAG)
                factory.onNativeSetFrame(1L, 0f, 0f, 320f, 220f)
                val bars = "{\"symbol\":\"AAPL\",\"bars\":[" +
                    "{\"t\":1,\"o\":10,\"h\":12,\"l\":9,\"c\":11,\"v\":100}," +
                    "{\"t\":2,\"o\":11,\"h\":13,\"l\":10,\"c\":12,\"v\":120}]}"
                factory.onNativeSetProps(
                    1L,
                    JSONObject().put("properties", JSONObject().put("attr:bars", bars)).toString(),
                )

                val view = factory.viewFor(1L)
                assertTrue("expected a CandlestickView, got ${view?.javaClass?.name}", view is CandlestickView)
                val chart = view as CandlestickView
                assertEquals(2, chart.barCount())
                assertEquals(NativeViewFactory.CUSTOM_CANDLESTICK_TAG, chart.tag)

                // A malformed update must not replace the last good payload.
                factory.onNativeSetProps(
                    1L,
                    JSONObject()
                        .put("properties", JSONObject().put("attr:bars", "{not-json"))
                        .toString(),
                )
                assertEquals(2, chart.barCount())

                // Angular-declared actions are retained for the refresh tap path.
                factory.onNativeCreateView(2L, "Button")
                factory.onNativeSetProps(
                    2L,
                    JSONObject()
                        .put("properties", JSONObject().put("attr:data-action", "refresh"))
                        .toString(),
                )
                assertEquals("refresh", factory.actionOf(2L))
                factory.clear()
            } catch (t: Throwable) {
                error.set(t)
            }
        }
        error.get()?.let { throw AssertionError("candlestick factory assertion failed", it) }
    }
}
