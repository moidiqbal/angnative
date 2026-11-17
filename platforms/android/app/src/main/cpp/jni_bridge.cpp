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
 * These call into the angnative-host-ffi C ABI via extern "C" declarations.
 */

#include <jni.h>
#include <cstdio>
#include <android/log.h>

// Include the angnative-host-ffi C API header
// This would be generated from the Rust crate's C ABI
extern "C" {
  // angnative-host-ffi C ABI declarations
  typedef void* AngNativeHost;

  AngNativeHost angnative_host_create();
  void angnative_host_destroy(AngNativeHost handle);
  void angnative_host_register_view_factory(
    AngNativeHost handle,
    void* create_view_cb,
    void* set_frame_cb,
    void* set_props_cb,
    void* set_text_cb,
    void* append_child_cb,
    void* insert_child_cb,
    void* remove_child_cb,
    void* destroy_view_cb,
    void* user_data
  );
  const char* angnative_host_process_batch(
    AngNativeHost handle,
    const char* batch_json,
    unsigned int* out_len
  );
  const char* angnative_host_run_layout(
    AngNativeHost handle,
    const char* request_json,
    unsigned int* out_len
  );
  const char* angnative_host_last_error(AngNativeHost handle);
  unsigned int angnative_host_node_count(AngNativeHost handle);
  const char* angnative_host_snapshot(AngNativeHost handle);
}

#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, "AngNative", __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, "AngNative", __VA_ARGS__)

// JNIEXPORT for nativeHostCreate
extern "C" JNIEXPORT jlong JNICALL
Java_com_angnative_app_JniBridge_nativeHostCreate(JNIEnv* env, jobject thiz) {
  AngNativeHost host = angnative_host_create();
  if (!host) {
    LOGE("Failed to create AngNative host");
    return 0;
  }
  LOGI("AngNative host created: %p", host);
  return reinterpret_cast<jlong>(host);
}

// JNIEXPORT for nativeHostDestroy
extern "C" JNIEXPORT void JNICALL
Java_com_angnative_app_JniBridge_nativeHostDestroy(JNIEnv* env, jobject thiz, jlong handle) {
  AngNativeHost h = reinterpret_cast<AngNativeHost>(handle);
  angnative_host_destroy(h);
  LOGI("AngNative host destroyed: %p", h);
}

// JNIEXPORT for nativeHostRegisterViewFactory
extern "C" JNIEXPORT void JNICALL
Java_com_angnative_app_JniBridge_nativeHostRegisterViewFactory(
  JNIEnv* env, jobject thiz, jlong handle, jlong callbackPtr) {

  AngNativeHost h = reinterpret_cast<AngNativeHost>(handle);

  // Register C callback function pointers
  // In production, these would be actual C functions that invoke Kotlin via JNI
  angnative_host_register_view_factory(
    h,
    nullptr, // create_view
    nullptr, // set_frame
    nullptr, // set_props
    nullptr, // set_text
    nullptr, // append_child
    nullptr, // insert_child
    nullptr, // remove_child
    nullptr, // destroy_view
    reinterpret_cast<void*>(callbackPtr) // user_data
  );

  LOGI("View factory registered for host: %p", h);
}

// JNIEXPORT for nativeHostProcessBatch
extern "C" JNIEXPORT jstring JNICALL
Java_com_angnative_app_JniBridge_nativeHostProcessBatch(
  JNIEnv* env, jobject thiz, jlong handle, jstring batchJson) {

  AngNativeHost h = reinterpret_cast<AngNativeHost>(handle);
  const char* batch = env->GetStringUTFChars(batchJson, nullptr);

  unsigned int outLen = 0;
  const char* result = angnative_host_process_batch(h, batch, &outLen);

  env->ReleaseStringUTFChars(batchJson, batch);

  if (!result) {
    return env->NewStringUTF("{\"error\":\"batch processing failed\"}");
  }

  jstring resultStr = env->NewStringUTF(result);
  // Free the C string allocated by Rust
  // In production, we'd need an angnative_host_free_string call
  return resultStr;
}

// JNIEXPORT for nativeHostRunLayout
extern "C" JNIEXPORT jstring JNICALL
Java_com_angnative_app_JniBridge_nativeHostRunLayout(
  JNIEnv* env, jobject thiz, jlong handle, jfloat width, jfloat height) {

  AngNativeHost h = reinterpret_cast<AngNativeHost>(handle);

  // Build layout request JSON
  char requestJson[256];
  snprintf(requestJson, sizeof(requestJson),
    "{\"root_id\":1,\"nodes\":[],\"available_width\":%f,\"available_height\":%f}",
    width, height);

  unsigned int outLen = 0;
  const char* result = angnative_host_run_layout(h, requestJson, &outLen);

  if (!result) {
    return env->NewStringUTF("{\"error\":\"layout failed\"}");
  }

  jstring resultStr = env->NewStringUTF(result);
  return resultStr;
}

// JNIEXPORT for nativeHostLastError
extern "C" JNIEXPORT jstring JNICALL
Java_com_angnative_app_JniBridge_nativeHostLastError(
  JNIEnv* env, jobject thiz, jlong handle) {

  AngNativeHost h = reinterpret_cast<AngNativeHost>(handle);
  const char* error = angnative_host_last_error(h);

  if (!error) {
    return env->NewStringUTF("{\"ok\":true}");
  }

  return env->NewStringUTF(error);
}

// JNIEXPORT for nativeHostNodeCount
extern "C" JNIEXPORT jint JNICALL
Java_com_angnative_app_JniBridge_nativeHostNodeCount(
  JNIEnv* env, jobject thiz, jlong handle) {

  AngNativeHost h = reinterpret_cast<AngNativeHost>(handle);
  return static_cast<jint>(angnative_host_node_count(h));
}

// JNIEXPORT for nativeHostSnapshot
extern "C" JNIEXPORT jstring JNICALL
Java_com_angnative_app_JniBridge_nativeHostSnapshot(
  JNIEnv* env, jobject thiz, jlong handle) {

  AngNativeHost h = reinterpret_cast<AngNativeHost>(handle);
  const char* snapshot = angnative_host_snapshot(h);

  if (!snapshot) {
    return env->NewStringUTF("{\"nodes\":[]}");
  }

  return env->NewStringUTF(snapshot);
}
