# OnePlus Open (CPH2551) physical-device verification — foldable aware.
# Collects screenshots + gfxinfo for both displays (inner + cover).
$ErrorActionPreference = 'Stop'
$adb = 'C:\Android\Sdk\platform-tools\adb.exe'
$dev = 'b6640c8f'
$repo = Split-Path -Parent $PSScriptRoot
$out = Join-Path $repo 'docs\benchmarks\physical'
New-Item -ItemType Directory -Force $out | Out-Null

$displayIds = @('4630946717172870531', '4630946316123810436')

function ScreenShot([string]$displayId, [string]$path) {
  for ($i=0; $i -lt 8; $i++) {
    & $adb -s $dev exec-out screencap -d $displayId -p > $path
    $b = [System.IO.File]::ReadAllBytes((Resolve-Path $path))
    if ($b.Length -gt 8 -and $b[0] -eq 0x89 -and $b[1] -eq 0x50) { return $true }
    Start-Sleep -Milliseconds 600
  }
  return $false
}

$apps = @(
  @{ n='material'; p='com.angnative.demo.material' },
  @{ n='primeng'; p='com.angnative.demo.primeng' },
  @{ n='ngzorro'; p='com.angnative.demo.ngzorro' },
  @{ n='taiga'; p='com.angnative.demo.taiga' },
  @{ n='spartan'; p='com.angnative.demo.spartan' },
  @{ n='ecosystem-grid'; p='com.angnative.demo.ecosystem.grid' },
  @{ n='ecosystem-charts'; p='com.angnative.demo.ecosystem.charts' },
  @{ n='ecosystem-core'; p='com.angnative.demo.ecosystem.core' },
  @{ n='ecosystem-richtext'; p='com.angnative.demo.ecosystem.richtext' },
  @{ n='ecosystem-calendar'; p='com.angnative.demo.ecosystem.calendar' }
)

Write-Output "model=$( & $adb -s $dev shell getprop ro.product.model ) android=$( & $adb -s $dev shell getprop ro.build.version.release )"
$results = @()
foreach ($a in $apps) {
  $apk = Get-ChildItem (Join-Path $repo 'dist\device-testing') -Filter "$($a.n)-*.apk" | Select-Object -First 1
  if (-not $apk) { continue }
  & $adb -s $dev install -r $apk.FullName 2>&1 | Select-String 'Success' | Select-Object -Last 1 | Out-Null
  & $adb -s $dev shell am force-stop $a.p 2>$null
  & $adb -s $dev shell dumpsys gfxinfo $a.p reset 2>$null | Out-Null
  & $adb -s $dev shell am start -n "$($a.p)/com.angnative.app.MainActivity" 2>$null | Out-Null
  Start-Sleep -Seconds 4

  # interaction burst
  for ($i=0; $i -lt 5; $i++) {
    & $adb -s $dev shell input tap 540 1200 2>$null
    & $adb -s $dev shell input swipe 540 1800 540 700 180 2>$null
    Start-Sleep -Milliseconds 120
  }
  Start-Sleep -Seconds 1

  $g = (& $adb -s $dev shell dumpsys gfxinfo $a.p framestats 2>&1 | Out-String)
  $frames = [regex]::Match($g,'Total frames rendered:\s*(\d+)').Groups[1].Value
  $jank = [regex]::Match($g,'Janky frames:\s*(\d+) \(([\d.]+)%\)').Groups[2].Value
  $p50  = [regex]::Match($g,'50th percentile:\s*(\d+)ms').Groups[1].Value
  $p95  = [regex]::Match($g,'95th percentile:\s*(\d+)ms').Groups[1].Value
  $mem  = (& $adb -s $dev shell dumpsys meminfo $a.p 2>&1 | Out-String)
  $pss  = [regex]::Match($mem,'TOTAL PSS:\s*(\d+)').Groups[1].Value
  $wv   = [regex]::Match($mem,'WebViews:\s*(\d+)').Groups[1].Value

  $ok = ScreenShot $displayIds[0] (Join-Path $out "$($a.n)-inner.png")
  Write-Output "$($a.n): frames=$frames janky=$jank% p50=${p50}ms p95=${p95}ms pss=${pss}KB webviews=$wv innerShot=$ok"
  $results += [ordered]@{ name=$a.n; package=$a.p; frames=$frames; janky_pct=$jank; p50_ms=$p50; p95_ms=$p95; pss_kb=$pss; webviews=$wv; inner_screenshot=$ok }
}
$results | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $out 'results.json')
Write-Output 'ONEPLUS VERIFICATION DONE'
