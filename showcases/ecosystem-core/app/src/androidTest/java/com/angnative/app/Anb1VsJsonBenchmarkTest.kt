package com.angnative.app

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertFalse
import org.junit.Test
import org.junit.runner.RunWith
import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.util.concurrent.atomic.AtomicReference
import kotlin.math.round
import org.json.JSONArray
import org.json.JSONObject

/**
 * Device JNI arm of the ANB1-vs-JSON measurement. Reported separately from the
 * host Rust arm; no cross-arm or superiority claim is made.
 *
 * The batch is built once on the instrumentation thread. Each measured call
 * gets a fresh Host (created/destroyed outside the timed region) and a direct
 * buffer allocated to exactly the ANB1 payload size, because the JNI entry
 * point reads `ByteBuffer.capacity()`, not `limit()`.
 *
 * Raw results are printed as one `ANB1_VS_JSON_RESULT:{...}` line so
 * `scripts/collect-android-benchmark-evidence.py` can retain them verbatim.
 */
@RunWith(AndroidJUnit4::class)
class Anb1VsJsonBenchmarkTest {
    private companion object {
        const val NODES = 120
        const val WARMUP = 16
        const val MEASURED = 64
    }

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

    /** Same command shape as the host Rust arm, encoded independently here. */
    private fun buildInputs(): Triple<String, ByteArray, Int> {
        val commands = mutableListOf<Pair<String, (BinWriter) -> Unit>>()
        for (id in 1..NODES) {
            val nodeType = if (id % 4 == 0) "Text" else "View"
            commands.add(
                "{\"kind\":\"CreateNode\",\"id\":$id,\"nodeType\":\"$nodeType\"}" to
                    { w: BinWriter -> w.u8(0x01); w.u64(id.toLong()); w.str(nodeType) }
            )
        }
        for (id in 1..NODES) {
            commands.add(
                "{\"kind\":\"SetStyle\",\"id\":$id,\"key\":\"color\",\"value\":\"#3b82f6\"}" to
                    { w: BinWriter -> w.u8(0x07); w.u64(id.toLong()); w.str("color"); w.str("#3b82f6") }
            )
            commands.add(
                "{\"kind\":\"SetProperty\",\"id\":$id,\"key\":\"opacity\",\"value\":0.9}" to
                    { w: BinWriter -> w.u8(0x02); w.u64(id.toLong()); w.str("opacity"); w.str("0.9") }
            )
            if (id % 4 == 0) {
                val text = "row $id — benchmark payload"
                commands.add(
                    "{\"kind\":\"SetTextContent\",\"id\":$id,\"text\":\"$text\"}" to
                        { w: BinWriter -> w.u8(0x03); w.u64(id.toLong()); w.str(text) }
                )
            }
        }
        for (id in 2..NODES) {
            commands.add(
                "{\"kind\":\"InsertChild\",\"parentId\":1,\"childId\":$id,\"index\":${id - 2}}" to
                    { w: BinWriter -> w.u8(0x04); w.u64(1); w.u64(id.toLong()); w.u32(id - 2) }
            )
        }

        val json = commands.joinToString(
            prefix = "{\"protocolVersion\":1,\"frameId\":1,\"timestampMs\":1700000000000,\"commands\":[",
            postfix = "]}",
        ) { it.first }
        val w = BinWriter()
        "ANB1".forEach { w.u8(it.code) }
        w.u16(1)
        w.u64(1)
        w.u64(1_700_000_000_000)
        w.u32(commands.size)
        commands.forEach { (_, encode) -> encode(w) }
        return Triple(json, w.bytes(), commands.size)
    }

    private fun statsJSON(values: List<Double>): JSONObject {
        val sorted = values.sorted()
        val arr = JSONArray()
        sorted.forEach { arr.put(round(it * 1000.0) / 1000.0) }
        return JSONObject()
            .put("samples", sorted.size)
            .put("min_us", round(sorted.first() * 1000.0) / 1000.0)
            .put("p50_us", round(sorted[sorted.size / 2] * 1000.0) / 1000.0)
            .put("p95_us", round(sorted[sorted.size * 95 / 100] * 1000.0) / 1000.0)
            .put("max_us", round(sorted.last() * 1000.0) / 1000.0)
            .put("mean_us", round(values.average() * 1000.0) / 1000.0)
            .put("raw_us", arr)
    }

    @Test
    fun jniJsonAndAnb1BatchesAreMeasuredSeparately() {
        val (json, anb1, commandCount) = buildInputs()
        val jsonSamples = mutableListOf<Double>()
        val anb1Samples = mutableListOf<Double>()
        val failure = AtomicReference<Throwable>()
        var opsJsonBytes = 0

        InstrumentationRegistry.getInstrumentation().runOnMainSync {
            val context = InstrumentationRegistry.getInstrumentation().targetContext
            fun sample(jsonFirst: Boolean) {
                val order = if (jsonFirst) listOf(true, false) else listOf(false, true)
                for (useJson in order) {
                    val bridge = JniBridge(context)
                    var initialized = false
                    try {
                        bridge.initHost()
                        initialized = true
                        val ops = if (useJson) {
                            val start = System.nanoTime()
                            val result = bridge.processBatch(json)
                            val elapsed = System.nanoTime() - start
                            if (useJson) jsonSamples.add(elapsed / 1000.0)
                            opsJsonBytes = result.length
                            result
                        } else {
                            // Exact-sized direct buffer: the JNI entry point uses
                            // capacity(), so limit() must not be smaller.
                            val buffer = ByteBuffer.allocateDirect(anb1.size)
                            buffer.put(anb1)
                            buffer.flip()
                            val start = System.nanoTime()
                            val result = bridge.processBatchDirect(buffer)
                            val elapsed = System.nanoTime() - start
                            anb1Samples.add(elapsed / 1000.0)
                            result
                        }
                        assertFalse("batch returned an error: $ops", ops.contains("\"error\""))
                    } finally {
                        if (initialized) bridge.destroyHost()
                    }
                }
            }

            try {
                repeat(WARMUP) { sample(it % 2 == 0) }
                jsonSamples.clear()
                anb1Samples.clear()
                repeat(MEASURED) { sample(it % 2 == 0) }
            } catch (t: Throwable) {
                failure.set(t)
            }
        }
        failure.get()?.let { throw it }

        val result = JSONObject()
            .put("title", "Device JNI ANB1 vs JSON batch processing (no superiority claim)")
            .put("platform", JSONObject()
                .put("manufacturer", android.os.Build.MANUFACTURER)
                .put("model", android.os.Build.MODEL)
                .put("device", android.os.Build.DEVICE)
                .put("release", android.os.Build.VERSION.RELEASE)
                .put("sdk", android.os.Build.VERSION.SDK_INT)
                .put("abi", android.os.Build.SUPPORTED_ABIS.firstOrNull() ?: "unknown"))
            .put("method", JSONObject()
                .put("inputs_precomputed_once", true)
                .put("fresh_host_per_iteration", true)
                .put("host_lifecycle_outside_timed_region", true)
                .put("timed_region", "JniBridge.processBatch / processBatchDirect on the Android main thread")
                .put("exact_sized_direct_buffer", true)
                .put("warmup_iterations", WARMUP)
                .put("measured_iterations", MEASURED)
                .put("alternating_order", true)
                .put("units", "microseconds from System.nanoTime"))
            .put("input", JSONObject()
                .put("commands", commandCount)
                .put("json_bytes", json.toByteArray(Charsets.UTF_8).size)
                .put("anb1_bytes", anb1.size)
                .put("ops_json_bytes", opsJsonBytes))
            .put("json", statsJSON(jsonSamples))
            .put("anb1", statsJSON(anb1Samples))
            .put("claim_boundary", JSONObject()
                .put("device_jni_arm", true)
                .put("host_rust_arm", "reported separately")
                .put("superiority_claim", "none")
                .put("end_to_end_zero_copy", "not claimed; decode produces an owned batch"))

        println("ANB1_VS_JSON_RESULT:$result")
    }
}
