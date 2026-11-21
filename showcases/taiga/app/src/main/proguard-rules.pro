# ProGuard rules for AngNative Android shell
-keep class com.angnative.app.** { *; }
-keep class io.angnative.** { *; }

# Keep JNI methods
-keepclasseswithmembers class com.angnative.app.JniBridge {
  native <methods>;
}

# Keep Angular-related classes if they exist
-keep class com.angnative.** { *; }
