package com.angnative.app

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference

/**
 * Device-side proof for the Android cross-thread JNI fix.
 *
 * The Rust staticlib linked into this APK is built with `cargo build --release`,
 * where `[profile.release] panic = "abort"`. If a wrong-thread JNI dispatch
 * reached `RuntimeBridge::assert_owner_thread()`, the whole app process would
 * abort and instrumentation would fail. Returning sentinels here therefore
 * proves the non-panicking C ABI owner-thread guard ran on an Android runtime,
 * not merely in host-side unit tests.
 */
@RunWith(AndroidJUnit4::class)
class RuntimeBridgeThreadAffinityTest {

    @Test
    fun wrongThreadDispatchLoadAndGetHostReturnSentinelsAndBridgeStaysAlive() {
        withBridge { bridge, handle, host ->
            val invoked = onWorkerThread {
                bridge.nativeRuntimeBridgeDispatchEvent(handle, 1L, "press", "{}")
            } as Int
            assertEquals(
                "DISPATCH_PANIC (u32::MAX) must surface as jint -1 on Android",
                -1,
                invoked,
            )

            val loadRc = onWorkerThread {
                bridge.nativeRuntimeBridgeLoadScript(handle, "globalThis.__wrongThread = 1;")
            } as Int
            assertEquals("LOAD_SCRIPT_PANIC must be 2", 2, loadRc)

            val borrowedHost = onWorkerThread { bridge.nativeRuntimeBridgeGetHost(handle) } as Long
            assertEquals("Wrong-thread get_host must return NULL", 0L, borrowedHost)

            // The bridge is still usable on the owner thread after all three
            // rejected calls. This also catches a caught-panic implementation:
            // under panic=abort a caught panic would have killed the process.
            assertEquals(
                0,
                bridge.nativeRuntimeBridgeLoadScript(handle, "globalThis.__ownerThread = 1;"),
            )
            assertEquals(host, bridge.nativeRuntimeBridgeGetHost(handle))
        }
    }

    @Test
    fun wrongThreadDestroyIsANoOpAndDoesNotAbortTheProcess() {
        val bridge = JniBridge(InstrumentationRegistry.getInstrumentation().targetContext)
        val handle = bridge.nativeRuntimeBridgeCreate()
        assertTrue("bridge creation must succeed", handle != 0L)
        val host = bridge.nativeRuntimeBridgeGetHost(handle)
        assertTrue("host handle must be non-null", host != 0L)

        // Intentionally do not destroy from this (owner) thread afterwards:
        // the wrong-thread destroy is a no-op, so the bridge is still alive and
        // must not be freed twice. Completion of this test is the assertion
        // that the process survived.
        onWorkerThread {
            bridge.nativeRuntimeBridgeDestroy(handle, host)
            Unit
        }
    }

    private fun withBridge(block: (JniBridge, Long, Long) -> Unit) {
        val bridge = JniBridge(InstrumentationRegistry.getInstrumentation().targetContext)
        val handle = bridge.nativeRuntimeBridgeCreate()
        assertTrue("bridge creation must succeed", handle != 0L)
        val host = bridge.nativeRuntimeBridgeGetHost(handle)
        assertTrue("host handle must be non-null", host != 0L)
        try {
            block(bridge, handle, host)
        } finally {
            bridge.nativeRuntimeBridgeDestroy(handle, host)
        }
    }

    /** Run [body] on a real Android worker thread and surface any exception. */
    private fun onWorkerThread(body: () -> Any?): Any? {
        val result = AtomicReference<Any?>()
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
        }, "angnative-wrong-thread-jni").start()

        assertTrue("worker thread timed out", done.await(10, TimeUnit.SECONDS))
        error.get()?.let { throw AssertionError("wrong-thread JNI call threw", it) }
        return result.get()
    }
}
