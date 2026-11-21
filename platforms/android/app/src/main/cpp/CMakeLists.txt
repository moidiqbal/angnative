cmake_minimum_required(VERSION 3.22.1)
project("angnative")

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# This file lives at <repo>/platforms/android/app/src/main/cpp.
# Walk up six levels to reach the repository root.
set(ANGNATIVE_REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../../../../../")
get_filename_component(ANGNATIVE_REPO_ROOT "${ANGNATIVE_REPO_ROOT}" ABSOLUTE)

# Make the exact C ABI header available to jni_bridge.cpp.
include_directories(
  "${ANGNATIVE_REPO_ROOT}/crates/angnative-host-ffi/include"
  ${ANDROID_NDK}/sources/android/native_app_glue
)

# Create shared library for JNI bridge
add_library(
  angnative
  SHARED
  jni_bridge.cpp
)

# Link the unified Rust library. libangnative_runtime_bridge.a is a staticlib
# that bundles host-ffi + host-engine + QuickJS + the runtime bridge into one
# archive (one owner, no duplicate symbols).
set(ANGNATIVE_RUST_LIB "")
if(EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/../jniLibs/${ANDROID_ABI}/libangnative_runtime_bridge.a")
  set(ANGNATIVE_RUST_LIB "${CMAKE_CURRENT_SOURCE_DIR}/../jniLibs/${ANDROID_ABI}/libangnative_runtime_bridge.a")
elseif(EXISTS "${ANGNATIVE_REPO_ROOT}/target/${ANDROID_ABI}/release/libangnative_runtime_bridge.a")
  set(ANGNATIVE_RUST_LIB "${ANGNATIVE_REPO_ROOT}/target/${ANDROID_ABI}/release/libangnative_runtime_bridge.a")
elseif(EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/../jniLibs/${ANDROID_ABI}/libangnative_host_ffi.a")
  # Legacy host-only fallback (no JS runtime round-trip).
  set(ANGNATIVE_RUST_LIB "${CMAKE_CURRENT_SOURCE_DIR}/../jniLibs/${ANDROID_ABI}/libangnative_host_ffi.a")
endif()

if(ANGNATIVE_RUST_LIB)
  add_library(angnative_rust STATIC IMPORTED)
  set_target_properties(angnative_rust PROPERTIES
    IMPORTED_LOCATION "${ANGNATIVE_RUST_LIB}"
  )
  target_link_libraries(angnative angnative_rust)
else()
  message(FATAL_ERROR "No AngNative Rust library for ${ANDROID_ABI}; run scripts/build-android-native.ps1")
endif()

target_link_libraries(angnative android log dl m)
