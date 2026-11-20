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

# Link against the AngNative Rust host library (prebuilt).
# Primary: library staged into jniLibs by the build script.
if(EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/../jniLibs/${ANDROID_ABI}/libangnative_host_ffi.a")
  add_library(angnative_host STATIC IMPORTED)
  set_target_properties(angnative_host PROPERTIES
    IMPORTED_LOCATION "${CMAKE_CURRENT_SOURCE_DIR}/../jniLibs/${ANDROID_ABI}/libangnative_host_ffi.a"
  )
  target_link_libraries(angnative angnative_host)
# Fallback: cargo output at the repository root (NOT platforms/android/crates).
elseif(EXISTS "${ANGNATIVE_REPO_ROOT}/crates/angnative-host-ffi/target/${ANDROID_ABI}/release/libangnative_host_ffi.a")
  add_library(angnative_host STATIC IMPORTED)
  set_target_properties(angnative_host PROPERTIES
    IMPORTED_LOCATION "${ANGNATIVE_REPO_ROOT}/crates/angnative-host-ffi/target/${ANDROID_ABI}/release/libangnative_host_ffi.a"
  )
  target_link_libraries(angnative angnative_host)
endif()

target_link_libraries(angnative android log)
