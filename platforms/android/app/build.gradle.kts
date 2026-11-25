plugins {
  // AGP 9+ compiles Kotlin natively — do NOT apply org.jetbrains.kotlin.android.
  id("com.android.application")
}

android {
  namespace = "com.angnative.app"
  // Latest stable toolchain: AGP 9.5 / Gradle 9.9 / JDK 25.
  compileSdk = 36
  buildToolsVersion = "37.0.0"
  ndkVersion = "27.2.12479018"

  defaultConfig {
    applicationId = "com.angnative.app"
    minSdk = 24
    targetSdk = 34
    versionCode = 1
    versionName = "0.1.0"

    // Device-side runtime-bridge tests (thread affinity, main-looper callbacks,
    // real AOT bundle boot). The Rust release profile uses panic="abort", so
    // these tests prove the C ABI sentinels actually ran on-device.
    testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"

    externalNativeBuild {
      cmake {
        arguments("-DANDROID_PLATFORM=android-24", "-DANDROID_STL=c++_shared")
        abiFilters("arm64-v8a", "armeabi-v7a", "x86_64")
      }
    }
  }

  externalNativeBuild {
    cmake {
      path = file("src/main/cpp/CMakeLists.txt")
      version = "3.22.1"
    }
  }

  buildTypes {
    release {
      isMinifyEnabled = false
      isShrinkResources = false
      proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
      signingConfig = signingConfigs.getByName("debug")
    }
    debug {
      isDebuggable = true
    }
  }

  compileOptions {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
  }

  // Kotlin jvmTarget now follows compileOptions.targetCompatibility (17).

  packaging {
    jniLibs {
      pickFirsts.add("lib/**/*.so")
    }
  }
}

// Stage the self-contained Angular bundles as app assets so this shell can
// boot Angular inside the QuickJS runtime. The counter bundle keeps its stable
// `angnative-bundle.js` name because the instrumentation suite asserts against
// it; the MarketPulse demo gets a separate asset and is selected at runtime.
val copyAngNativeBundle by tasks.registering(Copy::class) {
  val bundle = rootProject.file("../../dist/aot/quickjs-bundle.js")
  onlyIf { bundle.exists() }
  from(bundle)
  into(file("src/main/assets"))
  rename { "angnative-bundle.js" }
}
val copyMarketPulseBundle by tasks.registering(Copy::class) {
  val bundle = rootProject.file("../../dist/aot/market-pulse-bundle.js")
  onlyIf { bundle.exists() }
  from(bundle)
  into(file("src/main/assets"))
  rename { "market-pulse-bundle.js" }
}
tasks.named("preBuild") {
  dependsOn(copyAngNativeBundle, copyMarketPulseBundle)
}

dependencies {
  implementation("androidx.core:core-ktx:1.12.0")
  implementation("androidx.appcompat:appcompat:1.6.1")
  implementation("com.google.android.material:material:1.11.0")
  implementation("androidx.constraintlayout:constraintlayout:2.1.4")

  androidTestImplementation("androidx.test:runner:1.5.2")
  androidTestImplementation("androidx.test:core:1.5.0")
  androidTestImplementation("androidx.test.ext:junit:1.1.5")
}
