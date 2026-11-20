buildscript {
  repositories {
    google()
    mavenCentral()
  }
  dependencies {
    // AGP 9.x has built-in Kotlin support; no separate KGP classpath needed.
    classpath("com.android.tools.build:gradle:9.5.2")
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
