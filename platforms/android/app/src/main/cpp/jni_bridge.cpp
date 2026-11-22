/**
 * jni_bridge.cpp — JNI bridge between Android Kotlin and the Rust host.
 *
 * Implements the native methods declared in JniBridge.kt:
 *   - nativeHostCreate / nativeHostDestroy
 *   - nativeHostRegisterViewFactory
 *   - nativeHostProcessBatch
 *   - nativeHostRunLayout
 *   - nativeHostLastError
 *   - nativeHostNodeCount / nativeHostSnapshot
 *
 * The 8 ViewFactory callbacks are real C function pointers that call back into
 * the JniBridge instance (held as a JNI global ref inside ViewFactoryContext).
 * All strings returned by the Rust C ABI are freed with angnative_host_free_string.
 */

#include <jni.h>
#include <android/log.h>
#include <unordered_map>

#include "angnative_host_ffi.h"

#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, "AngNative", __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, "AngNative", __VA_ARGS__)

namespace {

/**
 * Heap-allocated context passed to Rust as the opaque `user_data` pointer.
 * It owns a JNI global reference to the JniBridge instance so callbacks can
 * invoke Kotlin methods even if Rust calls them from a non-JNI thread.
 */
struct ViewFactoryContext {
  JavaVM* vm = nullptr;
  jobject bridge = nullptr;  // JNI global ref (JniBridge instance)
  jmethodID createView = nullptr;
  jmethodID setFrame = nullptr;
  jmethodID setProps = nullptr;
  jmethodID setText = nullptr;
  jmethodID appendChild = nullptr;
  jmethodID insertChild = nullptr;
  jmethodID removeChild = nullptr;
  jmethodID destroyView = nullptr;
};

// Keyed by host handle so nativeHostDestroy can release the context + global ref.
std::unordered_map<jlong, ViewFactoryContext*> g_contexts;

void destroyContext(JNIEnv* env, ViewFactoryContext* ctx) {
  if (ctx == nullptr) return;
  if (env != nullptr && ctx->bridge != nullptr) {
    env->DeleteGlobalRef(ctx->bridge);
  }
  delete ctx;
}

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

}  // namespace

/* =========================================================================
 * ViewFactory callbacks (C linkage to match the typedefs in the header)
 * ========================================================================= */
extern "C" {

static void cb_create_view(uint64_t node_id, const char* view_type, void* user_data) {
  auto* ctx = static_cast<ViewFactoryContext*>(user_data);
  if (ctx == nullptr || view_type == nullptr) return;
  ScopedEnv se(ctx->vm);
  JNIEnv* env = se.get();
  if (env == nullptr) {
    LOGE("cb_create_view: could not obtain JNIEnv");
    return;
  }
  jstring jt = env->NewStringUTF(view_type);
  env->CallVoidMethod(ctx->bridge, ctx->createView, static_cast<jlong>(node_id), jt);
  env->DeleteLocalRef(jt);
  reportException(env, "onNativeCreateView");
}

static void cb_set_frame(uint64_t node_id, float x, float y, float width, float height,
                         void* user_data) {
  auto* ctx = static_cast<ViewFactoryContext*>(user_data);
  if (ctx == nullptr) return;
  ScopedEnv se(ctx->vm);
  JNIEnv* env = se.get();
  if (env == nullptr) {
    LOGE("cb_set_frame: could not obtain JNIEnv");
    return;
  }
  env->CallVoidMethod(ctx->bridge, ctx->setFrame, static_cast<jlong>(node_id),
                      static_cast<jfloat>(x), static_cast<jfloat>(y),
                      static_cast<jfloat>(width), static_cast<jfloat>(height));
  reportException(env, "onNativeSetFrame");
}

static void cb_set_props(uint64_t node_id, const char* props_json, void* user_data) {
  auto* ctx = static_cast<ViewFactoryContext*>(user_data);
  if (ctx == nullptr || props_json == nullptr) return;
  ScopedEnv se(ctx->vm);
  JNIEnv* env = se.get();
  if (env == nullptr) {
    LOGE("cb_set_props: could not obtain JNIEnv");
    return;
  }
  jstring jp = env->NewStringUTF(props_json);
  env->CallVoidMethod(ctx->bridge, ctx->setProps, static_cast<jlong>(node_id), jp);
  env->DeleteLocalRef(jp);
  reportException(env, "onNativeSetProps");
}

static void cb_set_text(uint64_t node_id, const char* text, void* user_data) {
  auto* ctx = static_cast<ViewFactoryContext*>(user_data);
  if (ctx == nullptr || text == nullptr) return;
  ScopedEnv se(ctx->vm);
  JNIEnv* env = se.get();
  if (env == nullptr) {
    LOGE("cb_set_text: could not obtain JNIEnv");
    return;
  }
  jstring jt = env->NewStringUTF(text);
  env->CallVoidMethod(ctx->bridge, ctx->setText, static_cast<jlong>(node_id), jt);
  env->DeleteLocalRef(jt);
  reportException(env, "onNativeSetText");
}

static void cb_append_child(uint64_t parent_id, uint64_t child_id, void* user_data) {
  auto* ctx = static_cast<ViewFactoryContext*>(user_data);
  if (ctx == nullptr) return;
  ScopedEnv se(ctx->vm);
  JNIEnv* env = se.get();
  if (env == nullptr) return;
  env->CallVoidMethod(ctx->bridge, ctx->appendChild, static_cast<jlong>(parent_id),
                      static_cast<jlong>(child_id));
  reportException(env, "onNativeAppendChild");
}

static void cb_insert_child(uint64_t parent_id, uint64_t child_id, uint32_t index,
                            void* user_data) {
  auto* ctx = static_cast<ViewFactoryContext*>(user_data);
  if (ctx == nullptr) return;
  ScopedEnv se(ctx->vm);
  JNIEnv* env = se.get();
  if (env == nullptr) return;
  env->CallVoidMethod(ctx->bridge, ctx->insertChild, static_cast<jlong>(parent_id),
                      static_cast<jlong>(child_id), static_cast<jint>(index));
  reportException(env, "onNativeInsertChild");
}

static void cb_remove_child(uint64_t parent_id, uint64_t child_id, void* user_data) {
  auto* ctx = static_cast<ViewFactoryContext*>(user_data);
  if (ctx == nullptr) return;
  ScopedEnv se(ctx->vm);
  JNIEnv* env = se.get();
  if (env == nullptr) return;
  env->CallVoidMethod(ctx->bridge, ctx->removeChild, static_cast<jlong>(parent_id),
                      static_cast<jlong>(child_id));
  reportException(env, "onNativeRemoveChild");
}

static void cb_destroy_view(uint64_t node_id, void* user_data) {
  auto* ctx = static_cast<ViewFactoryContext*>(user_data);
  if (ctx == nullptr) return;
  ScopedEnv se(ctx->vm);
  JNIEnv* env = se.get();
  if (env == nullptr) return;
  env->CallVoidMethod(ctx->bridge, ctx->destroyView, static_cast<jlong>(node_id));
  reportException(env, "onNativeDestroyView");
}

}  // extern "C"

/* =========================================================================
 * Exported JNI methods
 * ========================================================================= */

// DEPRECATED legacy host path. The 10 showcase shells use the unified
// RuntimeBridge (single owner of Host + QuickJS). Calling nativeHostDestroy on
// a bridge-owned handle is a double-free — never copy this pattern into a
// shell that uses nativeRuntimeBridgeCreate.
extern "C" JNIEXPORT [[deprecated("legacy host path; use angnative_runtime_bridge_create instead")]]
jlong JNICALL
Java_com_angnative_app_JniBridge_nativeHostCreate(JNIEnv* /*env*/, jobject /*thiz*/) {
  AngNativeHost* host = angnative_host_create();
  if (host == nullptr) {
    LOGE("Failed to create AngNative host");
    return 0;
  }
  LOGI("AngNative host created: %p", host);
  return reinterpret_cast<jlong>(host);
}

extern "C" JNIEXPORT [[deprecated("legacy host path; never call on a RuntimeBridge-owned Host")]]
void JNICALL
Java_com_angnative_app_JniBridge_nativeHostDestroy(JNIEnv* env, jobject /*thiz*/, jlong handle) {
  // Release the JNI global ref + context before destroying the Rust host.
  auto it = g_contexts.find(handle);
  if (it != g_contexts.end()) {
    destroyContext(env, it->second);
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
    destroyContext(env, it->second);
    g_contexts.erase(it);
  }

  auto* ctx = new ViewFactoryContext();
  env->GetJavaVM(&ctx->vm);
  ctx->bridge = env->NewGlobalRef(thiz);  // real global ref, not an identity hash

  jclass cls = env->GetObjectClass(thiz);
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
  }

  int rc = angnative_host_register_view_factory(
      h, cb_create_view, cb_set_frame, cb_set_props, cb_set_text,
      cb_append_child, cb_insert_child, cb_remove_child, cb_destroy_view, ctx);
  if (rc != 0) {
    LOGE("angnative_host_register_view_factory failed: %d", rc);
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
