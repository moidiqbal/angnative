# Build the Android shell with either verified toolchain profile.
#
#   pwsh -File scripts/build-android-stack.ps1 -Stack latest   # Gradle 9.9.2 / AGP 9.5.2 / JDK 25
#   pwsh -File scripts/build-android-stack.ps1 -Stack legacy   # Gradle 8.2   / AGP 8.2.0 / JDK 17
#
# Both Gradle versions, both JDKs, NDK 27.2 and CMake 3.22.1 are installed locally.
param(
  [ValidateSet('latest', 'legacy')]
  [string]$Stack = 'latest'
)

$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
$android = Join-Path $repo 'platforms\android'
$sdk = 'C:\Android\Sdk'
$profile = Join-Path $android "profiles\$Stack"

$jdk = if ($Stack -eq 'latest') {
  'C:\Program Files\Java\jdk-25.0.5.1'
} else {
  'C:\Program Files\Eclipse Adoptium\jdk-17.0.21.10002-hotspot'
}

# Apply the selected profile to the live Gradle files.
Copy-Item (Join-Path $profile 'gradle-wrapper.properties') (Join-Path $android 'gradle\wrapper\gradle-wrapper.properties') -Force
Copy-Item (Join-Path $profile 'build.gradle.kts') (Join-Path $android 'build.gradle.kts') -Force
Copy-Item (Join-Path $profile 'app-build.gradle.kts') (Join-Path $android 'app\build.gradle.kts') -Force

$env:ANDROID_HOME = $sdk
$env:ANDROID_SDK_ROOT = $sdk
$env:JAVA_HOME = $jdk
$env:PATH = "$jdk\bin;$sdk\platform-tools;$env:PATH"

Set-Location $android
Write-Output "Stack : $Stack"
Write-Output "Gradle: $((Select-String -Path 'gradle\wrapper\gradle-wrapper.properties' -Pattern 'distributionUrl').Line)"
Write-Output "JDK   : $jdk"
Write-Output "Java  : $(& java -version 2>&1 | Select-Object -First 1)"

& .\gradlew.bat assembleDebug --no-daemon
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Output "Android shell build succeeded with '$Stack' profile."
