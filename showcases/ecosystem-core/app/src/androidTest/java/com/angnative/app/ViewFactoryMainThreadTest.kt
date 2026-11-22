package com.angnative.app

import android.os.Looper
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.util.Collections
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference

/**
 * Device-side proof that off-main native entry points marshal Rust→Kotlin
 * ViewFactory callbacks onto Android's main looper.
 *
 * The sink records [Looper.myLooper()] inside each callback. UI-text assertions
 * alone are not enough: unmarshalled callbacks already produced correct text.
 */
@RunWith(AndroidJUnit4::class)
class ViewFactoryMainThreadTest {

    @Test
    fun viewFactoryCallbacksFromWorkerEntryAreMarshalledToMainLooper() {
        withMainBridge { bridge ->
            val sink = RecordingViewSink()
            bridge.viewFactory = sink

            // Deterministic host batch (legacy C ABI entry, but still a native
            // entry point wrapped by JniBridge). IDs are high to avoid colliding
            // with the Angular bundle's small node IDs in the next step.
            val opsJson = onWorkerThread { bridge.processBatch(ALL_OPS_BATCH) }
            assertTrue("batch failed: $opsJson", !opsJson.contains("\"error\""))
            assertTrue("batch must emit Create/Insert ops: $opsJson", opsJson.contains("Create"))
            assertTrue("batch must emit Insert ops: $opsJson", opsJson.contains("Insert"))

            // Canonical runtime path: the real AOT bundle arrives from a worker
            // thread; its synchronous callbacks must still land on main.
            val bundle = readAngularBundle()
            assertEquals("Angular bundle must load on main from a worker entry", 0, onWorkerThread {
                bridge.loadScript(bundle)
            })
            assertTrue("node count must be positive", bridge.nodeCount() > 0)

            val calls = sink.snapshot()
            val methods = calls.map { it.method }.toSet()
            // OpEmitter does not emit SetFrame (layout is applied by Kotlin),
            // Append, or Destroy for these commands. Assert what the real
            // native paths actually emit rather than claiming unreachable ops.
            val expected = setOf(
                "onNativeCreateView",
                "onNativeSetProps",
                "onNativeSetText",
                "onNativeInsertChild",
                "onNativeRemoveChild",
            )
            assertTrue(
                "Missing callbacks: ${expected - methods}; observed: $methods",
                expected.all { it in methods },
            )
            assertMainLooperOnly(calls)
        }
    }

    @Test
    fun dispatchNativeEventFromWorkerRunsOnMainAndReachesAngularListener() {
        withMainBridge { bridge ->
            val sink = RecordingViewSink()
            bridge.viewFactory = sink

            val bundle = readAngularBundle()
            assertEquals("Angular bundle must load on main from a worker entry", 0, onWorkerThread {
                bridge.loadScript(bundle)
            })

            // Node 17 is the "+" click host in the tracked AOT bundle; the
            // retained emulator logcat shows one listener on this node.
            val invoked = onWorkerThread { bridge.dispatchNativeEvent(17L, "click") }
            assertTrue("Expected a live Angular listener, got $invoked", invoked > 0)

            val calls = sink.snapshot()
            assertTrue("dispatch must emit view callbacks synchronously", calls.isNotEmpty())
            assertMainLooperOnly(calls)
        }
    }

    private fun assertMainLooperOnly(calls: List<SinkCall>) {
        val main = Looper.getMainLooper()
        calls.forEach { call ->
            assertEquals("${call.method} must run on main looper", main, call.looper)
            assertFalse(
                "${call.method} ran on ${call.threadName} instead of main",
                call.threadName == WORKER_THREAD_NAME,
            )
        }
    }

    private fun readAngularBundle(): String =
        InstrumentationRegistry.getInstrumentation().targetContext.assets
            .open("angnative-bundle.js").bufferedReader().use { it.readText() }

    /** Create/init on main so Rust's owner_thread is main, then clean up there. */
    private fun withMainBridge(block: (JniBridge) -> Unit) {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val bridgeRef = AtomicReference<JniBridge>()
        val errorRef = AtomicReference<Throwable>()
        instrumentation.runOnMainSync {
            try {
                val bridge = JniBridge(instrumentation.targetContext)
                bridgeRef.set(bridge)
                bridge.initHost()
            } catch (t: Throwable) {
                errorRef.set(t)
                bridgeRef.get()?.destroyHost()
            }
        }
        errorRef.get()?.let { throw AssertionError("main bridge setup failed", it) }
        val bridge = bridgeRef.get() ?: throw AssertionError("bridge was not created")
        try {
            block(bridge)
        } finally {
            instrumentation.runOnMainSync { bridge.destroyHost() }
        }
    }

    /** Run [body] on a real Android worker thread and surface any exception. */
    private fun <T> onWorkerThread(body: () -> T): T {
        val result = AtomicReference<T>()
        val error = AtomicReference<Throwable>()
        val done = CountDownLatch(1)
        Thread({
            try {
                result.set(body())
            } catch (t: Throwable) {
                error.set(t)
            } finally {
                done.countDown()
            }
        }, WORKER_THREAD_NAME).start()

        assertTrue("worker thread timed out", done.await(30, TimeUnit.SECONDS))
        error.get()?.let { throw AssertionError("worker entry call threw", it) }
        return result.get()
    }

    private data class SinkCall(
        val method: String,
        val looper: Looper?,
        val threadName: String,
        val args: String,
    )

    private class RecordingViewSink : NativeViewSink {
        private val calls = Collections.synchronizedList(mutableListOf<SinkCall>())

        private fun record(method: String, args: String) {
            calls.add(SinkCall(method, Looper.myLooper(), Thread.currentThread().name, args))
        }

        override fun onNativeCreateView(nodeId: Long, viewType: String) =
            record("onNativeCreateView", "nodeId=$nodeId viewType=$viewType")

        override fun onNativeSetFrame(nodeId: Long, x: Float, y: Float, width: Float, height: Float) =
            record("onNativeSetFrame", "nodeId=$nodeId")

        override fun onNativeSetProps(nodeId: Long, propsJson: String) =
            record("onNativeSetProps", "nodeId=$nodeId")

        override fun onNativeSetText(nodeId: Long, text: String) =
            record("onNativeSetText", "nodeId=$nodeId text=$text")

        override fun onNativeAppendChild(parentId: Long, childId: Long) =
            record("onNativeAppendChild", "parentId=$parentId childId=$childId")

        override fun onNativeInsertChild(parentId: Long, childId: Long, index: Int) =
            record("onNativeInsertChild", "parentId=$parentId childId=$childId index=$index")

        override fun onNativeRemoveChild(parentId: Long, childId: Long) =
            record("onNativeRemoveChild", "parentId=$parentId childId=$childId")

        override fun onNativeDestroyView(nodeId: Long) =
            record("onNativeDestroyView", "nodeId=$nodeId")

        fun snapshot(): List<SinkCall> = synchronized(calls) { calls.toList() }
    }

    private companion object {
        const val WORKER_THREAD_NAME = "angnative-viewfactory-worker"

        val ALL_OPS_BATCH = """
            {
              "protocolVersion": 1,
              "frameId": 0,
              "timestampMs": 0,
              "commands": [
                { "kind": "CreateNode", "id": 1001, "nodeType": "View" },
                { "kind": "CreateNode", "id": 1002, "nodeType": "Text" },
                { "kind": "SetProperty", "id": 1002, "key": "attr:test", "value": "worker" },
                { "kind": "SetStyle", "id": 1002, "key": "color", "value": "red" },
                { "kind": "SetTextContent", "id": 1002, "text": "worker-batch" },
                { "kind": "InsertChild", "parentId": 1001, "childId": 1002, "index": 0 },
                { "kind": "RemoveChild", "parentId": 1001, "childId": 1002 }
              ]
            }
        """.trimIndent()
    }
}
