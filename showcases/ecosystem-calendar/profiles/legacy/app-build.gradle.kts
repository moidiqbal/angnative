plugins {
  // Legacy stack needs the Kotlin Android plugin explicitly.
  id("com.android.application")
  id("org.jetbrains.kotlin.android")
}

android {
  namespace = "com.angnative.app"
  // Legacy stack: AGP 8.2 supports API 34; build-tools 34.0.0.
  compileSdk = 34
  buildToolsVersion = "34.0.0"
  ndkVersion = "27.2.12479018"

  defaultConfig {
    applicationId = "com.angnative.demo.ecosystem.calendar"
    minSdk = 24
    targetSdk = 34
    versionCode = 1
    versionName = "0.1.0"

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

  kotlinOptions {
    jvmTarget = "17"
  }

  packaging {
    jniLibs {
      pickFirsts.add("lib/**/*.so")
    }
  }
}

dependencies {
  implementation("androidx.core:core-ktx:1.12.0")
  implementation("androidx.appcompat:appcompat:1.6.1")
  implementation("com.google.android.material:material:1.11.0")
  implementation("androidx.constraintlayout:constraintlayout:2.1.4")
}

