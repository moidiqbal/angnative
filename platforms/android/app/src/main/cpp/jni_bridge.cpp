/**
 * jni_bridge.cpp — JNI bridge between Android Kotlin and the Rust host.
 *
 * Implements the native methods declared in JniBridge.kt:
 *   - nativeRuntimeBridgeCreate / nativeRuntimeBridgeDestroy / ...GetHost
 *   - nativeRuntimeBridgeDispatchEvent / nativeRuntimeBridgeLoadScript
 *   - nativeRuntimeBridgeHmrReload / nativeHostProcessBatchBytes
 *   - nativeRuntimeBridgeLifecycleState / Suspend / Resume / Stop
 *   - nativeRuntimeBridgeMetricsJson / nativeRuntimeBridgeLastErrorJson
 *   - nativeHostRegisterViewFactory
 *   - nativeHostProcessBatch / nativeHostProcessBatchBytes
 *   - nativeHostRunLayout
 *   - nativeHostLastError
 *   - nativeHostNodeCount / nativeHostSnapshot / nativeHostDispatchEvent
 *
 * NOTE: nativeHostCreate / nativeHostDestroy remain as LEGACY C exports but are
 * no longer declared by Kotlin — the runtime bridge owns the Host.
 *
 * The runtime bridge is identified by a validated u64 handle
 * (AngNativeRuntimeBridgeHandle); Kotlin keeps it as a Long. The 8 ViewFactory
 * callbacks are real C function pointers that call back into the JniBridge
 * instance (held as a JNI global ref inside ViewFactoryContext, owned by a
 * ScopedGlobalRef). All strings returned by the Rust C ABI are freed with the
 * matching free function for that ABI.
 */

#include <jni.h>
#include <android/log.h>
#include <unordered_map>

#include "angnative_host_ffi.h"

#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, "AngNative", __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, "AngNative", __VA_ARGS__)

namespace {

/** Gets a JNIEnv for the current thread, attaching/detaching if necessary. */
class ScopedEnv {
 public:
  explicit ScopedEnv(JavaVM* vm) : vm_(vm) {
    if (vm_ == nullptr) return;
    if (vm_->GetEnv(reinterpret_cast<void**>(&env_), JNI_VERSION_1_6) == JNI_EDETACHED) {
      if (vm_->AttachCurrentThread(&env_, nullptr) == JNI_OK) {
        attached_ = true;
      } else {
        env_ = nullptr;
      }
    }
  }
  ~ScopedEnv() {
    if (attached_ && vm_ != nullptr) vm_->DetachCurrentThread();
  }
  JNIEnv* get() const { return env_; }

 private:
  JavaVM* vm_ = nullptr;
  JNIEnv* env_ = nullptr;
  bool attached_ = false;
};

void reportException(JNIEnv* env, const char* where) {
  if (env != nullptr && env->ExceptionCheck()) {
    LOGE("JNI exception while calling %s", where);
    env->ExceptionDescribe();
    env->ExceptionClear();
  }
}

void throwJavaException(JNIEnv* env, const char* className, const char* message) {
  if (env == nullptr) return;
  jclass cls = env->FindClass(className);
  if (cls == nullptr) return;
  env->ThrowNew(cls, message);
  env->DeleteLocalRef(cls);
}

void freeRustString(const char* s) {
  if (s != nullptr) {
    // The Rust ABI allocates it as char*; the header exposes const char*.
    angnative_host_free_string(const_cast<char*>(s));
  }
}

/** Converts a Rust-owned C string into a Java String and frees the Rust string. */
jstring rustStringToJString(JNIEnv* env, const char* s) {
  jstring out = env->NewStringUTF(s);
  freeRustString(s);
  return out;
}

/** Runtime-bridge strings use their own allocator/free function. */
void freeRuntimeBridgeString(const char* s) {
  if (s != nullptr) {
    angnative_runtime_bridge_free_string(const_cast<char*>(s));
  }
}

jstring runtimeStringToJString(JNIEnv* env, const char* s) {
  if (s == nullptr) return nullptr;
  jstring out = env->NewStringUTF(s);
  freeRuntimeBridgeString(s);
  return out;
}

/**
 * RAII owner of a single JNI global reference. The JavaVM is retained so the
 * reference can be released from any thread (including Rust callback threads).
 */
class ScopedGlobalRef {
 public:
  ScopedGlobalRef() = default;
  ~ScopedGlobalRef() { reset(); }

  ScopedGlobalRef(const ScopedGlobalRef&) = delete;
  ScopedGlobalRef& operator=(const ScopedGlobalRef&) = delete;

  void reset(JNIEnv* env, JavaVM* vm, jobject obj) {
    reset();
    vm_ = vm;
    if (env != nullptr && obj != nullptr) {
      ref_ = env->NewGlobalRef(obj);
    }
  }

  void reset() {
    if (ref_ != nullptr) {
      if (vm_ != nullptr) {
        ScopedEnv se(vm_);
        JNIEnv* env = se.get();
        if (env != nullptr) env->DeleteGlobalRef(ref_);
      }
      ref_ = nullptr;
    }
    vm_ = nullptr;
  }

  jobject get() const { return ref_; }

 private:
  JavaVM* vm_ = nullptr;
  jobject ref_ = nullptr;
};

/**
 * Heap-allocated context passed to Rust as the opaque `user_data` pointer.
 * It owns a JNI global reference to the JniBridge instance so callbacks can
 * invoke Kotlin methods even if Rust calls them from a non-JNI thread.
 */
struct ViewFactoryContext {
  JavaVM* vm = nullptr;
  ScopedGlobalRef bridge;  // owns the JNI global ref (JniBridge instance)
  jmethodID createView = nullptr;
  jmethodID setFrame = nullptr;
  jmethodID setProps = nullptr;
  jmethodID setText = nullptr;
  jmethodID appendChild = nullptr;
  jmethodID insertChild = nullptr;
  jmethodID removeChild = nullptr;
  jmethodID destroyView = nullptr;
};

// Keyed by host handle so nativeRuntimeBridgeDestroy can release the context +
// JNI global ref (nativeHostDestroy only does this for the legacy path).
std::unordered_map<jlong, ViewFactoryContext*> g_contexts;

/** Deleting the context runs ScopedGlobalRef's destructor and releases the ref. */
void destroyContext(ViewFactoryContext* ctx) {
  delete ctx;
}

}  // namespace

/* =========================================================================
 * ViewFactory callbacks (C linkage to match the typedefs in the header)
 * ========================================================================= */
extern "C" {

static void cb_create_view(uint64_t node_id, const char* view_type, void* user_data) {
  auto* ctx = static_cast<ViewFactoryContext*>(user_data);
  if (ctx == nullptr || view_type == nullptr || ctx->bridge.get() == nullptr) return;
  ScopedEnv se(ctx->vm);
  JNIEnv* env = se.get();
  if (env == nullptr) {
    LOGE("cb_create_view: could not obtain JNIEnv");
    return;
  }
  jstring jt = env->NewStringUTF(view_type);
  env->CallVoidMethod(ctx->bridge.get(), ctx->createView, static_cast<jlong>(node_id), jt);
  env->DeleteLocalRef(jt);
  reportException(env, "onNativeCreateView");
}

static void cb_set_frame(uint64_t node_id, float x, float y, float width, float height,
                         void* user_data) {
  auto* ctx = static_cast<ViewFactoryContext*>(user_data);
  if (ctx == nullptr || ctx->bridge.get() == nullptr) return;
  ScopedEnv se(ctx->vm);
  JNIEnv* env = se.get();
  if (env == nullptr) {
    LOGE("cb_set_frame: could not obtain JNIEnv");
    return;
  }
  env->CallVoidMethod(ctx->bridge.get(), ctx->setFrame, static_cast<jlong>(node_id),
                      static_cast<jfloat>(x), static_cast<jfloat>(y),
                      static_cast<jfloat>(width), static_cast<jfloat>(height));
  reportException(env, "onNativeSetFrame");
}

static void cb_set_props(uint64_t node_id, const char* props_json, void* user_data) {
  auto* ctx = static_cast<ViewFactoryContext*>(user_data);
  if (ctx == nullptr || props_json == nullptr || ctx->bridge.get() == nullptr) return;
  ScopedEnv se(ctx->vm);
  JNIEnv* env = se.get();
  if (env == nullptr) {
    LOGE("cb_set_props: could not obtain JNIEnv");
    return;
  }
  jstring jp = env->NewStringUTF(props_json);
  env->CallVoidMethod(ctx->bridge.get(), ctx->setProps, static_cast<jlong>(node_id), jp);
  env->DeleteLocalRef(jp);
  reportException(env, "onNativeSetProps");
}

static void cb_set_text(uint64_t node_id, const char* text, void* user_data) {
  auto* ctx = static_cast<ViewFactoryContext*>(user_data);
  if (ctx == nullptr || text == nullptr || ctx->bridge.get() == nullptr) return;
  ScopedEnv se(ctx->vm);
  JNIEnv* env = se.get();
  if (env == nullptr) {
    LOGE("cb_set_text: could not obtain JNIEnv");
    return;
  }
  jstring jt = env->NewStringUTF(text);
  env->CallVoidMethod(ctx->bridge.get(), ctx->setText, static_cast<jlong>(node_id), jt);
  env->DeleteLocalRef(jt);
  reportException(env, "onNativeSetText");
}

static void cb_append_child(uint64_t parent_id, uint64_t child_id, void* user_data) {
  auto* ctx = static_cast<ViewFactoryContext*>(user_data);
  if (ctx == nullptr || ctx->bridge.get() == nullptr) return;
  ScopedEnv se(ctx->vm);
  JNIEnv* env = se.get();
  if (env == nullptr) return;
  env->CallVoidMethod(ctx->bridge.get(), ctx->appendChild, static_cast<jlong>(parent_id),
                      static_cast<jlong>(child_id));
  reportException(env, "onNativeAppendChild");
}

static void cb_insert_child(uint64_t parent_id, uint64_t child_id, uint32_t index,
                            void* user_data) {
  auto* ctx = static_cast<ViewFactoryContext*>(user_data);
  if (ctx == nullptr || ctx->bridge.get() == nullptr) return;
  ScopedEnv se(ctx->vm);
  JNIEnv* env = se.get();
  if (env == nullptr) return;
  env->CallVoidMethod(ctx->bridge.get(), ctx->insertChild, static_cast<jlong>(parent_id),
                      static_cast<jlong>(child_id), static_cast<jint>(index));
  reportException(env, "onNativeInsertChild");
}

static void cb_remove_child(uint64_t parent_id, uint64_t child_id, void* user_data) {
  auto* ctx = static_cast<ViewFactoryContext*>(user_data);
  if (ctx == nullptr || ctx->bridge.get() == nullptr) return;
  ScopedEnv se(ctx->vm);
  JNIEnv* env = se.get();
  if (env == nullptr) return;
  env->CallVoidMethod(ctx->bridge.get(), ctx->removeChild, static_cast<jlong>(parent_id),
                      static_cast<jlong>(child_id));
  reportException(env, "onNativeRemoveChild");
}

static void cb_destroy_view(uint64_t node_id, void* user_data) {
  auto* ctx = static_cast<ViewFactoryContext*>(user_data);
  if (ctx == nullptr || ctx->bridge.get() == nullptr) return;
  ScopedEnv se(ctx->vm);
  JNIEnv* env = se.get();
  if (env == nullptr) return;
  env->CallVoidMethod(ctx->bridge.get(), ctx->destroyView, static_cast<jlong>(node_id));
  reportException(env, "onNativeDestroyView");
}

}  // extern "C"

/* =========================================================================
 * Exported JNI methods
 * ========================================================================= */

extern "C" JNIEXPORT jlong JNICALL
Java_com_angnative_app_JniBridge_nativeHostCreate(JNIEnv* /*env*/, jobject /*thiz*/) {
  AngNativeHost* host = angnative_host_create();
  if (host == nullptr) {
    LOGE("Failed to create AngNative host");
    return 0;
  }
  LOGI("AngNative host created: %p", host);
  return reinterpret_cast<jlong>(host);
}

// LEGACY — only valid for hosts created by nativeHostCreate(). NEVER call this
// with a bridge-owned host handle: the runtime bridge owns the Host and will
// free it again (double-free). Kotlin no longer declares this entry point.
extern "C" JNIEXPORT void JNICALL
Java_com_angnative_app_JniBridge_nativeHostDestroy(JNIEnv* env, jobject /*thiz*/, jlong handle) {
  // Release the JNI global ref + context before destroying the Rust host.
  auto it = g_contexts.find(handle);
  if (it != g_contexts.end()) {
    destroyContext(it->second);
    g_contexts.erase(it);
  }
  AngNativeHost* h = reinterpret_cast<AngNativeHost*>(handle);
  angnative_host_destroy(h);
  LOGI("AngNative host destroyed: %p", h);
}

extern "C" JNIEXPORT void JNICALL
Java_com_angnative_app_JniBridge_nativeHostRegisterViewFactory(JNIEnv* env, jobject thiz,
                                                               jlong handle) {
  AngNativeHost* h = reinterpret_cast<AngNativeHost*>(handle);

  // Replace any previous context for this host (e.g. after reload).
  auto it = g_contexts.find(handle);
  if (it != g_contexts.end()) {
    destroyContext(it->second);
    g_contexts.erase(it);
  }

  auto* ctx = new ViewFactoryContext();
  env->GetJavaVM(&ctx->vm);
  ctx->bridge.reset(env, ctx->vm, thiz);  // real global ref, not an identity hash
  if (ctx->bridge.get() == nullptr) {
    LOGE("Could not allocate JNI global ref for ViewFactory context");
    reportException(env, "NewGlobalRef");
    destroyContext(ctx);  // not stored in g_contexts; release immediately
    throwJavaException(env, "java/lang/IllegalStateException",
                       "Could not create ViewFactory JNI global reference");
    return;
  }

  jclass cls = env->GetObjectClass(thiz);
  if (cls == nullptr) {
    reportException(env, "GetObjectClass");
    destroyContext(ctx);
    throwJavaException(env, "java/lang/IllegalStateException",
                       "Could not resolve JniBridge class");
    return;
  }
  auto mid = [&](const char* name, const char* sig) -> jmethodID {
    jmethodID m = env->GetMethodID(cls, name, sig);
    reportException(env, name);
    return m;
  };
  ctx->createView = mid("onNativeCreateView", "(JLjava/lang/String;)V");
  ctx->setFrame = mid("onNativeSetFrame", "(JFFFF)V");
  ctx->setProps = mid("onNativeSetProps", "(JLjava/lang/String;)V");
  ctx->setText = mid("onNativeSetText", "(JLjava/lang/String;)V");
  ctx->appendChild = mid("onNativeAppendChild", "(JJ)V");
  ctx->insertChild = mid("onNativeInsertChild", "(JJI)V");
  ctx->removeChild = mid("onNativeRemoveChild", "(JJ)V");
  ctx->destroyView = mid("onNativeDestroyView", "(J)V");
  env->DeleteLocalRef(cls);

  if (ctx->createView == nullptr || ctx->setFrame == nullptr || ctx->setProps == nullptr ||
      ctx->setText == nullptr || ctx->appendChild == nullptr || ctx->insertChild == nullptr ||
      ctx->removeChild == nullptr || ctx->destroyView == nullptr) {
    LOGE("One or more ViewFactory callback method IDs could not be resolved");
    destroyContext(ctx);  // release the global ref now; never publish this context
    throwJavaException(env, "java/lang/IllegalStateException",
                       "ViewFactory callback method IDs could not be resolved");
    return;
  }

  int rc = angnative_host_register_view_factory(
      h, cb_create_view, cb_set_frame, cb_set_props, cb_set_text,
      cb_append_child, cb_insert_child, cb_remove_child, cb_destroy_view, ctx);
  if (rc != 0) {
    LOGE("angnative_host_register_view_factory failed: %d", rc);
    destroyContext(ctx);  // registration did not take ownership: no double free
    throwJavaException(env, "java/lang/IllegalStateException",
                       "ViewFactory registration failed");
    return;
  }

  g_contexts[handle] = ctx;
  LOGI("View factory registered for host: %p (ctx=%p)", h, ctx);
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_angnative_app_JniBridge_nativeHostProcessBatch(JNIEnv* env, jobject /*thiz*/,
                                                        jlong handle, jstring batchJson) {
  AngNativeHost* h = reinterpret_cast<AngNativeHost*>(handle);
  const char* batch = env->GetStringUTFChars(batchJson, nullptr);
  if (batch == nullptr) return nullptr;

  unsigned int outLen = 0;
  const char* result = angnative_host_process_batch(h, batch, &outLen);
  env->ReleaseStringUTFChars(batchJson, batch);

  if (result == nullptr) {
    return env->NewStringUTF("{\"error\":\"batch processing failed\"}");
  }
  return rustStringToJString(env, result);  // frees the Rust string
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_angnative_app_JniBridge_nativeHostProcessBatchBytes(JNIEnv* env, jobject /*thiz*/,
                                                            jlong handle, jobject buffer) {
  AngNativeHost* h = reinterpret_cast<AngNativeHost*>(handle);
  void* address = env->GetDirectBufferAddress(buffer);
  jlong capacity = env->GetDirectBufferCapacity(buffer);
  if (address == nullptr || capacity <= 0) {
    return env->NewStringUTF("{\"error\":\"invalid direct buffer\"}");
  }

  unsigned int outLen = 0;
  // Borrowed for this synchronous call only: no global ref, no retained pointer.
  const char* result = angnative_host_process_batch_bytes(
      h, static_cast<const uint8_t*>(address), static_cast<size_t>(capacity), &outLen);
  if (result == nullptr) {
    return env->NewStringUTF("{\"error\":\"binary batch processing failed\"}");
  }
  return rustStringToJString(env, result);  // frees the Rust string
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_angnative_app_JniBridge_nativeHostRunLayout(JNIEnv* env, jobject thiz, jlong handle,
                                                     jfloat width, jfloat height) {
  AngNativeHost* h = reinterpret_cast<AngNativeHost*>(handle);

  // Build the layout request from the live shadow tree instead of a hardcoded
  // empty node list. Kotlin parses the snapshot and emits LayoutRequestJson.
  const char* snapshot = angnative_host_snapshot(h);
  if (snapshot == nullptr) {
    return env->NewStringUTF("{\"error\":\"snapshot failed\"}");
  }
  jstring jSnapshot = env->NewStringUTF(snapshot);
  freeRustString(snapshot);

  jclass cls = env->GetObjectClass(thiz);
  jmethodID buildRequest = env->GetMethodID(
      cls, "buildLayoutRequest", "(Ljava/lang/String;FF)Ljava/lang/String;");
  env->DeleteLocalRef(cls);
  if (buildRequest == nullptr) {
    reportException(env, "buildLayoutRequest");
    env->DeleteLocalRef(jSnapshot);
    return env->NewStringUTF("{\"error\":\"buildLayoutRequest missing\"}");
  }

  jstring jRequest = static_cast<jstring>(
      env->CallObjectMethod(thiz, buildRequest, jSnapshot, width, height));
  env->DeleteLocalRef(jSnapshot);
  reportException(env, "buildLayoutRequest");
  if (jRequest == nullptr) {
    return env->NewStringUTF("{\"error\":\"layout request build failed\"}");
  }

  const char* request = env->GetStringUTFChars(jRequest, nullptr);
  if (request == nullptr) {
    env->DeleteLocalRef(jRequest);
    return nullptr;
  }
  unsigned int outLen = 0;
  const char* result = angnative_host_run_layout(h, request, &outLen);
  env->ReleaseStringUTFChars(jRequest, request);
  env->DeleteLocalRef(jRequest);

  if (result == nullptr) {
    return env->NewStringUTF("{\"error\":\"layout failed\"}");
  }
  return rustStringToJString(env, result);  // frees the Rust string
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_angnative_app_JniBridge_nativeHostLastError(JNIEnv* env, jobject /*thiz*/, jlong handle) {
  AngNativeHost* h = reinterpret_cast<AngNativeHost*>(handle);
  const char* error = angnative_host_last_error(h);
  if (error == nullptr) {
    return env->NewStringUTF("{\"ok\":true}");
  }
  return rustStringToJString(env, error);  // frees the Rust string
}

extern "C" JNIEXPORT jint JNICALL
Java_com_angnative_app_JniBridge_nativeHostNodeCount(JNIEnv* /*env*/, jobject /*thiz*/,
                                                     jlong handle) {
  AngNativeHost* h = reinterpret_cast<AngNativeHost*>(handle);
  return static_cast<jint>(angnative_host_node_count(h));
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_angnative_app_JniBridge_nativeHostSnapshot(JNIEnv* env, jobject /*thiz*/, jlong handle) {
  AngNativeHost* h = reinterpret_cast<AngNativeHost*>(handle);
  const char* snapshot = angnative_host_snapshot(h);
  if (snapshot == nullptr) {
    return env->NewStringUTF("{\"error\":\"snapshot failed\"}");
  }
  return rustStringToJString(env, snapshot);  // frees the Rust string
}

extern "C" JNIEXPORT jint JNICALL
Java_com_angnative_app_JniBridge_nativeHostDispatchEvent(JNIEnv* env, jobject /*thiz*/,
                                                         jlong handle, jlong nodeId,
                                                         jstring eventName, jstring payloadJson) {
  AngNativeHost* h = reinterpret_cast<AngNativeHost*>(handle);
  if (h == nullptr) return 0;

  const char* event_name = env->GetStringUTFChars(eventName, nullptr);
  if (event_name == nullptr) return 0;

  const char* payload = payloadJson != nullptr
      ? env->GetStringUTFChars(payloadJson, nullptr)
      : nullptr;

  unsigned int invoked = angnative_host_dispatch_event(h, static_cast<uint64_t>(nodeId), event_name, payload);

  env->ReleaseStringUTFChars(eventName, event_name);
  if (payload != nullptr) {
    env->ReleaseStringUTFChars(payloadJson, payload);
  }

  return static_cast<jint>(invoked);
}

extern "C" JNIEXPORT jlong JNICALL
Java_com_angnative_app_JniBridge_nativeRuntimeBridgeCreate(JNIEnv* /*env*/, jobject /*thiz*/) {
  AngNativeRuntimeBridgeHandle bridge = angnative_runtime_bridge_create();
  if (bridge == 0) {
    LOGE("Failed to create AngNative runtime bridge");
    return 0;
  }
  LOGI("AngNative runtime bridge created: %llu", static_cast<unsigned long long>(bridge));
  return static_cast<jlong>(bridge);
}

extern "C" JNIEXPORT void JNICALL
Java_com_angnative_app_JniBridge_nativeRuntimeBridgeDestroy(JNIEnv* env, jobject /*thiz*/,
                                                            jlong handle, jlong hostHandle) {
  AngNativeRuntimeBridgeHandle b = static_cast<AngNativeRuntimeBridgeHandle>(handle);
  // The bridge owns the Host. Destroy it first so no ViewFactory callback can
  // fire with a dangling ViewFactoryContext, then release the JNI global ref.
  // This also fixes the leak where the old Kotlin destroyHost() dropped the
  // bridge but never cleared g_contexts / the global ref (review P1-08).
  angnative_runtime_bridge_destroy(b);
  if (hostHandle != 0) {
    auto it = g_contexts.find(hostHandle);
    if (it != g_contexts.end()) {
      destroyContext(it->second);
      g_contexts.erase(it);
    }
  }
  LOGI("AngNative runtime bridge destroyed: %llu", static_cast<unsigned long long>(b));
}

extern "C" JNIEXPORT jint JNICALL
Java_com_angnative_app_JniBridge_nativeRuntimeBridgeDispatchEvent(JNIEnv* env, jobject /*thiz*/,
                                                                  jlong handle, jlong nodeId,
                                                                  jstring eventName, jstring payloadJson) {
  AngNativeRuntimeBridgeHandle b = static_cast<AngNativeRuntimeBridgeHandle>(handle);
  if (b == 0) return 0;

  const char* event_name = env->GetStringUTFChars(eventName, nullptr);
  if (event_name == nullptr) return 0;

  const char* payload = payloadJson != nullptr
      ? env->GetStringUTFChars(payloadJson, nullptr)
      : nullptr;

  unsigned int invoked = angnative_runtime_bridge_dispatch_event(b, static_cast<uint64_t>(nodeId), event_name, payload);

  env->ReleaseStringUTFChars(eventName, event_name);
  if (payload != nullptr) {
    env->ReleaseStringUTFChars(payloadJson, payload);
  }

  return static_cast<jint>(invoked);
}

extern "C" JNIEXPORT jint JNICALL
Java_com_angnative_app_JniBridge_nativeRuntimeBridgeLoadScript(JNIEnv* env, jobject /*thiz*/,
                                                              jlong handle, jstring source) {
  AngNativeRuntimeBridgeHandle b = static_cast<AngNativeRuntimeBridgeHandle>(handle);
  if (b == 0 || source == nullptr) return 1;
  const char* script = env->GetStringUTFChars(source, nullptr);
  if (script == nullptr) return 1;
  int rc = angnative_runtime_bridge_load_script(b, script);
  env->ReleaseStringUTFChars(source, script);
  if (rc != 0) LOGE("Failed to load JS bundle into runtime bridge (rc=%d)", rc);
  return static_cast<jint>(rc);
}

extern "C" JNIEXPORT jint JNICALL
Java_com_angnative_app_JniBridge_nativeRuntimeBridgeHmrReload(JNIEnv* env, jobject /*thiz*/,
                                                             jlong handle, jstring moduleId,
                                                             jstring source) {
  AngNativeRuntimeBridgeHandle b = static_cast<AngNativeRuntimeBridgeHandle>(handle);
  if (b == 0 || moduleId == nullptr || source == nullptr) return 1;
  const char* id = env->GetStringUTFChars(moduleId, nullptr);
  const char* script = env->GetStringUTFChars(source, nullptr);
  if (id == nullptr || script == nullptr) {
    if (id != nullptr) env->ReleaseStringUTFChars(moduleId, id);
    if (script != nullptr) env->ReleaseStringUTFChars(source, script);
    return 1;
  }
  int rc = angnative_runtime_bridge_hmr_reload(b, id, script);
  env->ReleaseStringUTFChars(moduleId, id);
  env->ReleaseStringUTFChars(source, script);
  return static_cast<jint>(rc);
}

extern "C" JNIEXPORT jlong JNICALL
Java_com_angnative_app_JniBridge_nativeRuntimeBridgeGetHost(JNIEnv* /*env*/, jobject /*thiz*/, jlong handle) {
  AngNativeRuntimeBridgeHandle b = static_cast<AngNativeRuntimeBridgeHandle>(handle);
  if (b == 0) return 0;

  AngNativeHost* h = angnative_runtime_bridge_get_host(b);
  if (h == nullptr) return 0;

  return reinterpret_cast<jlong>(h);
}

extern "C" JNIEXPORT jint JNICALL
Java_com_angnative_app_JniBridge_nativeRuntimeBridgeLifecycleState(JNIEnv* /*env*/,
                                                                  jobject /*thiz*/,
                                                                  jlong handle) {
  AngNativeRuntimeBridgeHandle b = static_cast<AngNativeRuntimeBridgeHandle>(handle);
  return static_cast<jint>(angnative_runtime_bridge_lifecycle_state(b));
}

extern "C" JNIEXPORT jint JNICALL
Java_com_angnative_app_JniBridge_nativeRuntimeBridgeSuspend(JNIEnv* /*env*/, jobject /*thiz*/,
                                                           jlong handle) {
  AngNativeRuntimeBridgeHandle b = static_cast<AngNativeRuntimeBridgeHandle>(handle);
  return static_cast<jint>(angnative_runtime_bridge_suspend(b));
}

extern "C" JNIEXPORT jint JNICALL
Java_com_angnative_app_JniBridge_nativeRuntimeBridgeResume(JNIEnv* /*env*/, jobject /*thiz*/,
                                                          jlong handle) {
  AngNativeRuntimeBridgeHandle b = static_cast<AngNativeRuntimeBridgeHandle>(handle);
  return static_cast<jint>(angnative_runtime_bridge_resume(b));
}

extern "C" JNIEXPORT jint JNICALL
Java_com_angnative_app_JniBridge_nativeRuntimeBridgeStop(JNIEnv* /*env*/, jobject /*thiz*/,
                                                        jlong handle) {
  AngNativeRuntimeBridgeHandle b = static_cast<AngNativeRuntimeBridgeHandle>(handle);
  return static_cast<jint>(angnative_runtime_bridge_stop(b));
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_angnative_app_JniBridge_nativeRuntimeBridgeMetricsJson(JNIEnv* env, jobject /*thiz*/,
                                                               jlong handle) {
  AngNativeRuntimeBridgeHandle b = static_cast<AngNativeRuntimeBridgeHandle>(handle);
  const char* json = angnative_runtime_bridge_metrics_json(b);
  return runtimeStringToJString(env, json);
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_angnative_app_JniBridge_nativeRuntimeBridgeLastErrorJson(JNIEnv* env, jobject /*thiz*/,
                                                                  jlong handle) {
  AngNativeRuntimeBridgeHandle b = static_cast<AngNativeRuntimeBridgeHandle>(handle);
  const char* json = angnative_runtime_bridge_last_error_json(b);
  return runtimeStringToJString(env, json);
}

/** Drain host-capability tasks (async fetch/timers) queued by the JS shims. */
extern "C" JNIEXPORT jstring JNICALL
Java_com_angnative_app_JniBridge_nativeRuntimeBridgeTakeHostTasks(JNIEnv* env, jobject /*thiz*/,
                                                                  jlong handle) {
  AngNativeRuntimeBridgeHandle b = static_cast<AngNativeRuntimeBridgeHandle>(handle);
  if (b == 0) return nullptr;
  // The runtime bridge allocator owns this string; runtimeStringToJString frees
  // it with angnative_runtime_bridge_free_string after the Java copy is made.
  const char* json = angnative_runtime_bridge_take_host_tasks(b);
  return runtimeStringToJString(env, json);
}
