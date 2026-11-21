# Phase 5/6 evidence collector: startup + memory + APK size for an installed showcase.
# Usage: pwsh -File scripts/measure-showcase.ps1 -Package com.angnative.demo.ecosystem.grid -Name grid
param(
  [Parameter(Mandatory=$true)][string]$Package,
  [Parameter(Mandatory=$true)][string]$Name
)
$ErrorActionPreference = 'Stop'
$adb = 'C:\Android\Sdk\platform-tools\adb.exe'

# Bail out fast if no device is online — never hang the caller.
$devices = (& $adb devices 2>$null | Out-String)
if ($devices -notmatch 'emulator-\d+\s+device') {
  Write-Output "SKIP $Name — no online emulator/device"
  exit 0
}

$outDir = Join-Path (Split-Path -Parent $PSScriptRoot) 'docs\benchmarks'
New-Item -ItemType Directory -Force $outDir | Out-Null

function Get-Pid($pkg) {
  $v = (& $adb shell pidof $pkg 2>$null | Out-String).Trim()
  if ($v -match '^\d+$') { return $v }
  return $null
}

& $adb shell am force-stop $Package 2>$null | Out-Null
Start-Sleep -Milliseconds 500

# Cold start: measure until the process appears and the window is drawn.
$sw = [System.Diagnostics.Stopwatch]::StartNew()
& $adb shell am start -n "$Package/com.angnative.app.MainActivity" 2>$null | Out-Null
$pidApp = $null
for ($i=0; $i -lt 100; $i++) {
  Start-Sleep -Milliseconds 100
  $pidApp = Get-Pid $Package
  if ($pidApp) { break }
}
$processMs = $sw.ElapsedMilliseconds
# Extra settle for first frame
Start-Sleep -Seconds 3
$totalMs = $sw.ElapsedMilliseconds

$mem = (& $adb shell dumpsys meminfo $Package 2>$null | Out-String)
$pss = if ($mem -match 'TOTAL PSS:\s*(\d+)') { [int]$Matches[1] } else { -1 }
$wv  = if ($mem -match 'WebViews:\s*(\d+)') { [int]$Matches[1] } else { -1 }

$apk = Join-Path (Split-Path -Parent $PSScriptRoot) "dist\showcases\$Name\app-debug.apk"
$apkBytes = if (Test-Path $apk) { (Get-Item $apk).Length } else { -1 }

$result = [ordered]@{
  name              = $Name
  package           = $Package
  measured_at       = (Get-Date).ToString('o')
  device            = (& $adb shell getprop ro.product.model 2>$null | Out-String).Trim()
  android_api       = (& $adb shell getprop ro.build.version.sdk 2>$null | Out-String).Trim()
  cold_process_ms   = $processMs
  cold_total_ms     = $totalMs
  pss_kb            = $pss
  webviews          = $wv
  apk_bytes         = $apkBytes
  sample_count      = 1
  note              = 'Single cold-start sample on emulator; not a frame-pacing benchmark.'
}
$json = $result | ConvertTo-Json -Depth 4
$file = Join-Path $outDir "$Name.json"
Set-Content $file $json
Write-Output $json
Write-Output "saved: $file"
