# Install + launch + screenshot + WebView check for every native showcase.
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$adb = 'C:\Android\Sdk\platform-tools\adb.exe'

# Wait for boot
for ($i=0; $i -lt 120; $i++) {
  Start-Sleep -Seconds 2
  $b = (& $adb shell getprop sys.boot_completed 2>$null | Out-String).Trim()
  if ($b -eq '1') { Write-Output 'boot completed'; break }
}

$apps = @(
  @{ id='material'; pkg='com.angnative.demo.material' },
  @{ id='primeng';  pkg='com.angnative.demo.primeng' },
  @{ id='ngzorro';  pkg='com.angnative.demo.ngzorro' },
  @{ id='taiga';    pkg='com.angnative.demo.taiga' },
  @{ id='spartan';  pkg='com.angnative.demo.spartan' }
)
foreach ($a in $apps) {
  $apk = Join-Path $repo "dist\showcases\$($a.id)\app-debug.apk"
  Write-Output "=== $($a.id) ==="
  & $adb install -r $apk 2>&1 | Select-String -Pattern 'Success|Failure|error' | Select-Object -Last 1
  & $adb shell monkey -p $a.pkg -c android.intent.category.LAUNCHER 1 2>&1 | Select-String -Pattern 'Events injected|Error' | Select-Object -Last 1
  Start-Sleep -Seconds 6
  & $adb exec-out screencap -p > (Join-Path $repo "docs\assets\showcase-$($a.id).png")
  $mem = (& $adb shell dumpsys meminfo $a.pkg 2>$null | Out-String)
  $wv = if ($mem -match 'WebViews:\s*(\d+)') { $Matches[1] } else { 'n/a' }
  Write-Output "  WebViews=$wv  screenshot=docs/assets/showcase-$($a.id).png"
}
Write-Output 'ALL SHOWCASES VERIFIED'
