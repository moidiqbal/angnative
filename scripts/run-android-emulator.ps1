# Create/start an Android 34 x86_64 emulator, install the AngNative demo APK,
# launch it and capture a screenshot + filtered logcat.
$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
$sdk = 'C:\Android\Sdk'
$adb = Join-Path $sdk 'platform-tools\adb.exe'
$emulator = Join-Path $sdk 'emulator\emulator.exe'
$avdManager = Join-Path $sdk 'cmdline-tools\latest\bin\avdmanager.bat'
$image = 'system-images;android-34;google_apis;x86_64'
$apk = Join-Path $repo 'platforms\android\app\build\outputs\apk\debug\app-debug.apk'
$avdName = 'AngNative34'

$env:ANDROID_HOME = $sdk
$env:ANDROID_SDK_ROOT = $sdk
$env:JAVA_HOME = 'C:\Program Files\Java\jdk-25.0.5.1'
$env:PATH = "$sdk\platform-tools;$env:PATH"

if (-not (Test-Path $apk)) { throw "APK not found: $apk" }
if (-not (Test-Path $emulator)) { throw "Emulator not installed at $emulator" }

if (-not (Test-Path "$env:USERPROFILE\.android\avd\$avdName.avd")) {
  Write-Output "Creating AVD $avdName ..."
  & $avdManager create avd --name $avdName --package $image --force
}

Write-Output "Starting emulator $avdName ..."
$proc = Start-Process -FilePath $emulator `
  -ArgumentList @('-avd', $avdName, '-no-snapshot', '-no-boot-anim', '-gpu', 'swiftshader_indirect', '-port', '5554') `
  -PassThru

$ready = $false
for ($i = 0; $i -lt 180; $i++) {
  Start-Sleep -Seconds 2
  $devices = & $adb devices 2>$null
  if ($devices -match 'emulator-5554\s+device') { $ready = $true; break }
  if ($i % 15 -eq 0) { Write-Output "waiting for emulator... ($($i*2)s)" }
}
if (-not $ready) { throw "Emulator did not become ready" }

# Wait for the system to finish booting.
for ($i = 0; $i -lt 120; $i++) {
  $boot = & $adb shell getprop sys.boot_completed 2>$null
  if (($boot | Out-String).Trim() -eq '1') { break }
  Start-Sleep -Seconds 2
}

Write-Output "Installing APK ..."
& $adb install -r $apk
Write-Output "Launching activity ..."
& $adb shell am start -n com.angnative.app/.MainActivity
Start-Sleep -Seconds 12

$shotDir = Join-Path $repo 'docs\assets'
New-Item -ItemType Directory -Force $shotDir | Out-Null
$shot = Join-Path $shotDir 'android-demo.png'
& $adb exec-out screencap -p > $shot
Write-Output "Screenshot: $shot"

Write-Output "=== logcat (AngNative) ==="
& $adb logcat -d -t 200 -s AngNative:* 2>$null

Write-Output "Emulator PID: $($proc.Id) (leave it running for inspection)"
