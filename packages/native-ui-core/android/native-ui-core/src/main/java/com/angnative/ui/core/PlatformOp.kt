/**
 * PlatformOp.kt — typed representation of the eight Rust PlatformOps
 * (crates/angnative-host-view/src/ops.rs).
 *
 * The live FFI path does not serialize these: the C++ bridge resolves the eight
 * `onNative*` JNI callbacks directly and the host invokes them one by one.
 * This typed model + decoder is for hosts/tests that replay a serialized op
 * sequence (e.g. `angnative_host_get_ops` output or a recorded batch).
 *
 * Serde's default externally-tagged shape is understood:
 *   {"Create":{"node_id":1,"view_type":"Text"}}
 * as well as the flat form {"op":"Create","node_id":1,"view_type":"Text"}.
 */
package com.angnative.ui.core

import org.json.JSONArray
import org.json.JSONObject

sealed class PlatformOp {
  data class Create(val nodeId: Long, val viewType: String) : PlatformOp()
  data class SetFrame(
    val nodeId: Long,
    val x: Float,
    val y: Float,
    val width: Float,
    val height: Float
  ) : PlatformOp()

  /** [propsJson] is the serialized `ViewProps`: {text, styles, properties}. */
  data class SetProps(val nodeId: Long, val propsJson: String) : PlatformOp()
  data class SetText(val nodeId: Long, val text: String) : PlatformOp()
  data class Append(val parentId: Long, val childId: Long) : PlatformOp()
  data class Insert(val parentId: Long, val childId: Long, val index: Int) : PlatformOp()
  data class Remove(val parentId: Long, val childId: Long) : PlatformOp()
  data class Destroy(val nodeId: Long) : PlatformOp()

  companion object {
    /** Decode one op; returns null for unknown kinds or malformed input. */
    fun fromJson(input: String): PlatformOp? = try {
      val outer = JSONObject(input.trim())
      val firstKey = outer.keys().asSequence().firstOrNull()
      val isTagged = outer.length() == 1 && firstKey != "op"
      val kind = if (isTagged) firstKey!! else outer.optString("op")
      val body = if (isTagged) outer.optJSONObject(kind) ?: JSONObject() else outer

      when (kind) {
        "Create" -> Create(body.optLong("node_id"), body.optString("view_type"))
        "SetFrame" -> SetFrame(
          body.optLong("node_id"),
          body.optDouble("x").toFloat(),
          body.optDouble("y").toFloat(),
          body.optDouble("width").toFloat(),
          body.optDouble("height").toFloat()
        )
        "SetProps" -> {
          val props = body.optJSONObject("props") ?: body.optJSONObject("properties")
          SetProps(body.optLong("node_id"), (props ?: JSONObject()).toString())
        }
        "SetText" -> SetText(body.optLong("node_id"), body.optString("text"))
        "Append" -> Append(body.optLong("parent_id"), body.optLong("child_id"))
        "Insert" -> Insert(
          body.optLong("parent_id"),
          body.optLong("child_id"),
          body.optInt("index")
        )
        "Remove" -> Remove(body.optLong("parent_id"), body.optLong("child_id"))
        "Destroy" -> Destroy(body.optLong("node_id"))
        else -> null
      }
    } catch (e: Exception) {
      null
    }

    /**
     * Decode a serialized op sequence: a top-level JSON array, or an object
     * with an "ops" / "operations" array. Ops are returned in order.
     */
    fun decodeBatch(json: String): List<PlatformOp> = try {
      val elements: JSONArray = if (json.trim().startsWith("[")) {
        JSONArray(json.trim())
      } else {
        val root = JSONObject(json.trim())
        root.optJSONArray("ops") ?: root.optJSONArray("operations") ?: return emptyList()
      }
      (0 until elements.length()).mapNotNull { i -> fromJson(elements.optString(i)) }
    } catch (e: Exception) {
      emptyList()
    }
  }
}

/** Apply a decoded op to a factory using the same callbacks the JNI bridge uses. */
fun NativeViewFactory.apply(op: PlatformOp) {
  when (op) {
    is PlatformOp.Create -> onNativeCreateView(op.nodeId, op.viewType)
    is PlatformOp.SetFrame -> onNativeSetFrame(op.nodeId, op.x, op.y, op.width, op.height)
    is PlatformOp.SetProps -> onNativeSetProps(op.nodeId, op.propsJson)
    is PlatformOp.SetText -> onNativeSetText(op.nodeId, op.text)
    is PlatformOp.Append -> onNativeAppendChild(op.parentId, op.childId)
    is PlatformOp.Insert -> onNativeInsertChild(op.parentId, op.childId, op.index)
    is PlatformOp.Remove -> onNativeRemoveChild(op.parentId, op.childId)
    is PlatformOp.Destroy -> onNativeDestroyView(op.nodeId)
  }
}
