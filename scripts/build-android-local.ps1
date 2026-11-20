# Local Windows helper: build the Android shell with the SDK/NDK installed via winget/sdkmanager.
# Usage: pwsh -File scripts/build-android-local.ps1
$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
$sdk = 'C:\Android\Sdk'
$jdk = 'C:\Program Files\Java\jdk-25.0.5.1'

$env:ANDROID_HOME = $sdk
$env:ANDROID_SDK_ROOT = $sdk
$env:JAVA_HOME = $jdk
$env:PATH = "$jdk\bin;$sdk\platform-tools;$env:PATH"

Set-Location (Join-Path $repo 'platforms\android')
Write-Output "SDK : $sdk"
Write-Output "JDK : $jdk"
Write-Output "Java: $(& java -version 2>&1 | Select-Object -First 1)"

& .\gradlew.bat assembleDebug --no-daemon --stacktrace
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Output 'Android shell build succeeded.'
