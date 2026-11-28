package com.angnative.app

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.util.concurrent.atomic.AtomicReference

/**
 * Physical-device proof for the three priority-#6+ features:
 *  1. the versioned ANB1 binary mutation protocol through borrowed JNI memory,
 *  2. zero-copy direct-buffer borrow with a caller-owned buffer, and
 *  3. same-realm QuickJS HMR with snapshot/restore state preservation.
 *
 * Retained output and the exact device/APK hash live in
 * `docs/evidence/HMR-ZEROCOPY-BINARY-PROTOCOL.md`.
 */
@RunWith(AndroidJUnit4::class)
class HmrZeroCopyBinaryTest {

    private class BinWriter {
        private val out = ByteArrayOutputStream()
        fun u8(value: Int) { out.write(value and 0xff) }
        fun u16(value: Int) { u8(value); u8(value shr 8) }
        fun u32(value: Int) { for (shift in 0 until 32 step 8) u8(value shr shift) }
        fun u64(value: Long) { for (shift in 0 until 64 step 8) u8((value shr shift).toInt()) }
        fun str(value: String) {
            val bytes = value.toByteArray(Charsets.UTF_8)
            u32(bytes.size)
            out.write(bytes, 0, bytes.size)
        }
        fun bytes(): ByteArray = out.toByteArray()
    }

    private fun binaryBatch(text: String): ByteArray {
        val w = BinWriter()
        "ANB1".forEach { w.u8(it.code) }
        w.u16(1)      // BINARY_VERSION
        w.u64(1)      // frameId
        w.u64(0)      // timestampMs
        w.u32(4)      // command count
        w.u8(0x01); w.u64(1); w.str("View")          // CreateNode 1
        w.u8(0x01); w.u64(2); w.str("Text")          // CreateNode 2
        w.u8(0x04); w.u64(1); w.u64(2); w.u32(0)     // InsertChild
        w.u8(0x03); w.u64(2); w.str(text)            // SetTextContent
        return w.bytes()
    }

    private fun textForNode(bridge: JniBridge, id: Long): String =
        bridge.snapshot().nodes.firstOrNull { it.id == id }?.textContent
            ?: throw AssertionError("node $id missing from host snapshot")

    @Test
    fun borrowedBinaryBufferAndSameRealmHmrAreVerifiedOnDevice() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val failure = AtomicReference<Throwable>()
        instrumentation.runOnMainSync {
            val context = instrumentation.targetContext
            val bridge = JniBridge(context)
            var initialized = false
            try {
                val container = android.widget.FrameLayout(context)
                bridge.initHost()
                initialized = true
                val factory = NativeViewFactory(context, container)
                bridge.viewFactory = factory
                factory.setRootNode(1L)

                // --- 1 + 2: ANB1 binary protocol through a borrowed direct buffer ---
                val bytes = binaryBatch("borrowed")
                val direct = ByteBuffer.allocateDirect(bytes.size)
                direct.put(bytes)
                direct.flip()
                val ops = bridge.processBatchDirect(direct)
                assertFalse("binary batch returned an error: $ops", ops.contains("\"error\""))
                assertTrue("binary ops must be emitted: $ops", ops.contains("Create"))
                assertEquals(2, bridge.nodeCount())
                assertEquals("borrowed", textForNode(bridge, 2L))

                // Reuse the caller-owned buffer after the call. Rust decoded into
                // an owned batch, so the earlier tree must be unchanged.
                direct.position(0)
                while (direct.hasRemaining()) direct.put(0.toByte())
                assertEquals("borrowed", textForNode(bridge, 2L))

                // --- 3: same-realm HMR with counter-style state preservation ---
                val initial = """
                    (function () {
                      globalThis.__hmrMarker = 424242;
                      function make(label) {
                        return {
                          count: 0, label: label, disposed: false,
                          snapshot: function () { return { count: this.count }; },
                          restore: function (s) {
                            if (s && typeof s.count === 'number') this.count = s.count;
                          },
                          dispose: function () { this.disposed = true; }
                        };
                      }
                      globalThis.__makeCounter = make;
                      __angnativeHmr.register('counter', function () { return make('v1'); }, {
                        snapshot: function (i) { return i.snapshot(); },
                        restore: function (i, s) { i.restore(s); },
                        dispose: function (i) { i.dispose(); }
                      });
                      globalThis.__oldInstance = __angnativeHmr.instance('counter');
                      __angnativeHmr.instance('counter').count = 7;
                      na_dispatchBatch(JSON.stringify({
                        protocolVersion: 1, frameId: 1, timestampMs: 0,
                        commands: [{ kind: 'SetTextContent', id: 2, text: '7' }]
                      }));
                    })();
                """.trimIndent()
                assertEquals("initial HMR module must load", 0, bridge.loadScript(initial))

                val reloaded = """
                    (function () {
                      globalThis.__makeCounter = function (label) {
                        return {
                          count: 0, label: label, disposed: false,
                          snapshot: function () { return { count: this.count }; },
                          restore: function (s) {
                            if (s && typeof s.count === 'number') this.count = s.count;
                          },
                          dispose: function () { this.disposed = true; }
                        };
                      };
                      __angnativeHmr.replace('counter', function () {
                        return globalThis.__makeCounter('v2');
                      }, {
                        snapshot: function (i) { return i.snapshot(); },
                        restore: function (i, s) { i.restore(s); },
                        dispose: function (i) { i.dispose(); }
                      });
                      var restored = __angnativeHmr.instance('counter').count;
                      na_dispatchBatch(JSON.stringify({
                        protocolVersion: 1, frameId: 2, timestampMs: 0,
                        commands: [{ kind: 'SetTextContent', id: 2,
                          text: String(restored) + '|' + String(__hmrMarker) + '|' +
                                String(__oldInstance.disposed) + '|' +
                                String(__angnativeHmr.generation('counter')) }]
                      }));
                    })();
                """.trimIndent()
                assertEquals("HMR reload must succeed", 0, bridge.hmrReload("counter", reloaded))

                // 7 = restored local state; 424242 = same JS realm; true =
                // dispose hook ran on the old instance; 2 = generation advanced.
                assertEquals(
                    "7|424242|true|2",
                    textForNode(bridge, 2L),
                )
            } catch (t: Throwable) {
                failure.set(t)
            } finally {
                if (initialized) bridge.destroyHost()
            }
        }
        failure.get()?.let { throw it }
    }
}
