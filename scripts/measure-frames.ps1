# Phase 6 frame-pacing evidence: gfxinfo frame stats after a short interaction burst.
# Usage: pwsh -File scripts/measure-frames.ps1 -Package com.angnative.demo.ecosystem.grid -Name ecosystem-grid
param(
  [Parameter(Mandatory=$true)][string]$Package,
  [Parameter(Mandatory=$true)][string]$Name
)
$adb = 'C:\Android\Sdk\platform-tools\adb.exe'
$devices = (& $adb devices 2>$null | Out-String)
if ($devices -notmatch 'emulator-\d+\s+device') { Write-Output "SKIP $Name — no online device"; exit 0 }

& $adb shell dumpsys gfxinfo $Package reset 2>$null | Out-Null
& $adb shell am force-stop $Package 2>$null | Out-Null
Start-Sleep -Milliseconds 500
& $adb shell am start -n "$Package/com.angnative.app.MainActivity" 2>$null | Out-Null
Start-Sleep -Seconds 4

# Interaction burst: scroll the grid / charts area so frames are produced.
for ($i=0; $i -lt 4; $i++) {
  & $adb shell input swipe 540 1700 540 600 250 2>$null | Out-Null
  Start-Sleep -Milliseconds 300
}
Start-Sleep -Seconds 1

$stats = (& $adb shell dumpsys gfxinfo $Package framestats 2>$null | Out-String)
$lines = $stats -split "`n"
$frames = @($lines | Where-Object { $_ -match '^\s*\d+\s+\d' })
$statline = @($lines | Where-Object { $_ -match '\(Janky frames\)|Janky frames:|50th percentile|95th percentile' }) | Select-Object -First 6

$outDir = Join-Path (Split-Path -Parent $PSScriptRoot) 'docs\benchmarks'
New-Item -ItemType Directory -Force $outDir | Out-Null
$result = [ordered]@{
  name = $Name
  package = $Package
  measured_at = (Get-Date).ToString('o')
  device = (& $adb shell getprop ro.product.model 2>$null | Out-String).Trim()
  frames_sampled = $frames.Count
  gfxinfo_summary = $statline
  note = 'Emulator software rendering (SwiftShader); not representative of physical GPU performance.'
}
$file = Join-Path $outDir "$Name-frames.json"
Set-Content $file ($result | ConvertTo-Json -Depth 4)
Write-Output ($result | ConvertTo-Json -Depth 4)
Write-Output "saved: $file"
