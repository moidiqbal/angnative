cmake_minimum_required(VERSION 3.22.1)
project("angnative")

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Find the AngNative Rust host static library
# This would be built by cargo and placed in the jniLibs directory
set(ANGNATIVE_HOST_LIB "angnative-host")

# Include paths for JNI
include_directories(${ANDROID_NDK}/sources/android/native_app_glue)

# Create shared library for JNI bridge
add_library(
  angnative
  SHARED
  jni_bridge.cpp
)

# Link against the AngNative Rust host library (prebuilt)
if(EXISTS "${CMAKE_SOURCE_DIR}/../jniLibs/${ANDROID_ABI}/libangnative_host_ffi.a")
  add_library(angnative_host STATIC IMPORTED)
  set_target_properties(angnative_host PROPERTIES
    IMPORTED_LOCATION "${CMAKE_SOURCE_DIR}/../jniLibs/${ANDROID_ABI}/libangnative_host_ffi.a"
  )
  target_link_libraries(angnative angnative_host)
elseif(EXISTS "${CMAKE_SOURCE_DIR}/../../../../crates/angnative-host-ffi/target/${ANDROID_ABI}/release/libangnative_host_ffi.a")
  add_library(angnative_host STATIC IMPORTED)
  set_target_properties(angnative_host PROPERTIES
    IMPORTED_LOCATION "${CMAKE_SOURCE_DIR}/../../../../crates/angnative-host-ffi/target/${ANDROID_ABI}/release/libangnative_host_ffi.a"
  )
  target_link_libraries(angnative angnative_host)
endif()

target_link_libraries(angnative android log)
