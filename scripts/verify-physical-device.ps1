# Physical-device verification: install all 10 showcase APKs, launch each,
# collect screenshot + gfxinfo frame stats + memory. Targets the OnePlus Open.
$ErrorActionPreference = 'Stop'
$adb = 'C:\Android\Sdk\platform-tools\adb.exe'
$device = 'b6640c8f'
$repo = Split-Path -Parent $PSScriptRoot
$apkDir = Join-Path $repo 'dist\device-testing'
$outDir = Join-Path $repo 'docs\benchmarks\physical'
New-Item -ItemType Directory -Force $outDir | Out-Null

function ADB([string[]]$args) { & $adb -s $device @args 2>$null }

Write-Output "=== DEVICE ==="
ADB @('shell','getprop','ro.product.model')
ADB @('shell','getprop','ro.build.version.release')
ADB @('shell','wm','size')
$fold = (ADB @('shell','wm','size') | Out-String).Trim()
Write-Output "display: $fold"

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

$results = @()
foreach ($a in $apps) {
  $apk = Get-ChildItem $apkDir -Filter "$($a.n)-*.apk" | Select-Object -First 1
  if (-not $apk) { Write-Output "SKIP $($a.n) — apk missing"; continue }
  Write-Output "=== $($a.n) ==="
  ADB @('install','-r',$apk.FullName) | Select-String -Pattern 'Success|Failure' | Select-Object -Last 1
  ADB @('shell','am','force-stop',$a.p)
  ADB @('shell','dumpsys','gfxinfo',$a.p,'reset')
  ADB @('shell','am','start',"$($a.p)/com.angnative.app.MainActivity") | Out-Null
  Start-Sleep -Seconds 4

  $top = (ADB @('shell','dumpsys','activity','activities') | Select-String -Pattern 'topResumedActivity' | Select-Object -First 1 | Out-String)
  $foreground = $top -match $a.p

  # interaction burst
  for ($i=0; $i -lt 5; $i++) {
    ADB @('shell','input','tap','540','1200')
    ADB @('shell','input','swipe','540','1800','540','700','180')
    Start-Sleep -Milliseconds 120
  }
  Start-Sleep -Seconds 1

  $g = (ADB @('shell','dumpsys','gfxinfo',$a.p,'framestats') | Out-String)
  $frames = [regex]::Match($g,'Total frames rendered:\s*(\d+)').Groups[1].Value
  $jank = [regex]::Match($g,'Janky frames:\s*(\d+) \(([\d.]+)%\)').Groups[2].Value
  $p50  = [regex]::Match($g,'50th percentile:\s*(\d+)ms').Groups[1].Value
  $p95  = [regex]::Match($g,'95th percentile:\s*(\d+)ms').Groups[1].Value

  $mem = (ADB @('shell','dumpsys','meminfo',$a.p) | Out-String)
  $pss = [regex]::Match($mem,'TOTAL PSS:\s*(\d+)').Groups[1].Value
  $wv  = [regex]::Match($mem,'WebViews:\s*(\d+)').Groups[1].Value

  ADB @('exec-out','screencap','-p') > (Join-Path $outDir "$($a.n).png")

  $crash = ((ADB @('logcat','-d','-t','80') | Select-String -Pattern 'FATAL EXCEPTION' | Out-String).Trim() -ne '')
  $results += [ordered]@{
    name=$a.n; package=$a.p; foreground=$foreground; frames=$frames; janky_pct=$jank
    p50_ms=$p50; p95_ms=$p95; pss_kb=$pss; webviews=$wv; fatal_exception=$crash
  }
  Write-Output "  fg=$foreground frames=$frames janky=$jank% p50=${p50}ms p95=${p95}ms pss=${pss}KB webviews=$wv crash=$crash"
}
$results | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $outDir 'results.json')
Write-Output 'PHYSICAL DEVICE VERIFICATION DONE'
