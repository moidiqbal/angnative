buildscript {
  repositories {
    google()
    mavenCentral()
  }
  dependencies {
    // Legacy stable stack: AGP 8.2 + Kotlin Gradle plugin 1.9.20 + JDK 17.
    classpath("com.android.tools.build:gradle:8.2.0")
    classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.20")
  }
}

allprojects {
  repositories {
    google()
    mavenCentral()
  }
}

tasks.register<Delete>("clean") {
  delete(rootProject.layout.buildDirectory)
}
