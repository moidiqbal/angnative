package com.angnative.app

import android.widget.FrameLayout
import android.widget.ProgressBar
import android.widget.Switch
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.util.concurrent.atomic.AtomicReference

/**
 * Device-side checks for the remaining Android/JNI closure work:
 * canonical tagged custom primitives, stale validated-u64-handle sentinels,
 * and the main-thread marshalled lifecycle wrappers.
 *
 * These are instrumentation tests written for an Android runtime; no
 * physical-device result is claimed by this source file.
 */
@RunWith(AndroidJUnit4::class)
class RuntimeBridgeLifecycleTest {

    @Test
    fun taggedCustomPrimitivesMaterializeWithCanonicalTags() = runOnMain {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val container = FrameLayout(context)
        val factory = NativeViewFactory(context, container)

        factory.setRootNode(1L)
        factory.onNativeCreateView(1L, NativeViewFactory.CUSTOM_LIST_TAG)
        factory.onNativeCreateView(2L, NativeViewFactory.CUSTOM_SWITCH_TAG)
        factory.onNativeCreateView(3L, NativeViewFactory.CUSTOM_ACTIVITY_INDICATOR_TAG)
        // Legacy aliases must keep working and carry the same canonical tag.
        factory.onNativeCreateView(4L, "Switch")
        factory.onNativeCreateView(5L, "ActivityIndicator")

        val list = factory.viewFor(1L)
        assertTrue("na-list must materialize as a FrameLayout", list is FrameLayout)
        assertEquals(NativeViewFactory.CUSTOM_LIST_TAG, list?.tag)

        val toggle = factory.viewFor(2L)
        assertTrue("na-switch must materialize as a Switch", toggle is Switch)
        assertEquals(NativeViewFactory.CUSTOM_SWITCH_TAG, toggle?.tag)

        val spinner = factory.viewFor(3L)
        assertTrue("na-activity-indicator must materialize as a ProgressBar", spinner is ProgressBar)
        assertTrue("indicator must be indeterminate", (spinner as ProgressBar).isIndeterminate)
        assertEquals(NativeViewFactory.CUSTOM_ACTIVITY_INDICATOR_TAG, spinner.tag)

        assertTrue("Switch alias must still materialize", factory.viewFor(4L) is Switch)
        assertEquals(NativeViewFactory.CUSTOM_SWITCH_TAG, factory.viewFor(4L)?.tag)
        assertTrue("ActivityIndicator alias must still materialize", factory.viewFor(5L) is ProgressBar)
        assertEquals(
            NativeViewFactory.CUSTOM_ACTIVITY_INDICATOR_TAG,
            factory.viewFor(5L)?.tag,
        )
    }

    @Test
    fun staleU64HandleReturnsSafeSentinels() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val bridge = JniBridge(context)
        val handleRef = AtomicReference<Long>()

        // The Rust bridge is thread-affine: create and destroy on main.
        runOnMain { handleRef.set(bridge.nativeRuntimeBridgeCreate()) }
        val handle = handleRef.get() ?: throw AssertionError("bridge handle was not created")
        assertTrue("bridge creation must succeed", handle != 0L)
        runOnMain { bridge.nativeRuntimeBridgeDestroy(handle, 0L) }

        // u64 generations are monotonic: this handle is stale, never reused.
        runOnMain {
            assertEquals(-1, bridge.nativeRuntimeBridgeLifecycleState(handle))
            assertEquals(4, bridge.nativeRuntimeBridgeSuspend(handle))
            assertEquals(4, bridge.nativeRuntimeBridgeResume(handle))
            assertEquals(4, bridge.nativeRuntimeBridgeStop(handle))
            assertEquals(0, bridge.nativeRuntimeBridgeDispatchEvent(handle, 1L, "click", "{}"))
            assertEquals(0L, bridge.nativeRuntimeBridgeGetHost(handle))
            assertNull(bridge.nativeRuntimeBridgeMetricsJson(handle))
            assertNull(bridge.nativeRuntimeBridgeLastErrorJson(handle))
        }

        // Kotlin wrappers guard a zero handle before it ever reaches JNI.
        assertEquals(-1, bridge.lifecycleState())
        assertEquals(4, bridge.suspendRuntime())
        assertEquals(4, bridge.resumeRuntime())
        assertEquals(4, bridge.stopRuntime())
        assertEquals("{\"error\":\"metrics unavailable\"}", bridge.metricsJson())
        assertEquals(4, JSONObject(bridge.lastErrorJson()).getInt("code"))
    }

    @Test
    fun lifecycleWrappersTrackSuspendResumeAndStop() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val bridge = JniBridge(context)
        val handleRef = AtomicReference<Long>()

        runOnMain { handleRef.set(bridge.nativeRuntimeBridgeCreate()) }
        val handle = handleRef.get() ?: throw AssertionError("bridge handle was not created")
        assertTrue("bridge creation must succeed", handle != 0L)

        try {
            // A fresh bridge is Created -> Initializing -> Ready -> Running.
            assertEquals(3, bridge.lifecycleState())
            assertEquals(0, bridge.suspendRuntime())
            assertEquals(4, bridge.lifecycleState())
            assertEquals(0, bridge.resumeRuntime())
            assertEquals(3, bridge.lifecycleState())

            val metrics = JSONObject(bridge.metricsJson())
            assertTrue("metrics must report the engine", metrics.has("engine_name"))
            val error = JSONObject(bridge.lastErrorJson())
            assertEquals(0, error.getInt("code"))

            assertEquals(0, bridge.stopRuntime())
            assertEquals(6, bridge.lifecycleState())
        } finally {
            runOnMain { bridge.nativeRuntimeBridgeDestroy(handle, 0L) }
        }
    }

    /** Capture main-thread assertion failures and rethrow them to the test thread. */
    private fun runOnMain(body: () -> Unit) {
        val error = AtomicReference<Throwable>()
        InstrumentationRegistry.getInstrumentation().runOnMainSync {
            try {
                body()
            } catch (t: Throwable) {
                error.set(t)
            }
        }
        error.get()?.let { throw AssertionError("main-thread check failed", it) }
    }
}
