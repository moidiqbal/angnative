plugins {
  // AGP 9+ compiles Kotlin natively — do NOT apply org.jetbrains.kotlin.android.
  id("com.android.library")
}

android {
  namespace = "com.angnative.ui.core"
  // Matches the toolchain used by the showcase apps: AGP 9.5.2 / Gradle 9.9.2 / JDK 25.
  compileSdk = 36
  buildToolsVersion = "37.0.0"

  defaultConfig {
    minSdk = 24
  }

  compileOptions {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
  }

  testOptions {
    // org.json methods on android.jar are stubs in local JVM tests; the library's
    // pure codec tests never call them, but default values keep the harness happy.
    unitTests.isReturnDefaultValues = true
  }
}

dependencies {
  testImplementation("junit:junit:4.13.2")
}
