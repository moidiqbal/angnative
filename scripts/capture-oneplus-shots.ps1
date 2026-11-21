# Valid screenshots on OnePlus Open: adb screencap emits a "Multiple displays"
# warning before PNG bytes on stdout. This strips the prefix and saves a real PNG.
$ErrorActionPreference = 'Stop'
$adb = 'C:\Android\Sdk\platform-tools\adb.exe'
$dev = 'b6640c8f'
$repo = Split-Path -Parent $PSScriptRoot
$outDir = Join-Path $repo 'docs\benchmarks\physical'
New-Item -ItemType Directory -Force $outDir | Out-Null

function Save-ValidScreenshot([string]$path) {
  $raw = Join-Path $env:TEMP 'angnative-scap.raw'
  & $adb -s $dev exec-out screencap -p > $raw
  $b = [System.IO.File]::ReadAllBytes($raw)
  $start = -1
  for ($i=0; $i -lt [Math]::Min(3000, $b.Length-8); $i++) {
    if ($b[$i] -eq 0x89 -and $b[$i+1] -eq 0x50 -and $b[$i+2] -eq 0x4E -and $b[$i+3] -eq 0x47) { $start=$i; break }
  }
  if ($start -lt 0) { return $false }
  $png = $b[$start..($b.Length-1)]
  [System.IO.File]::WriteAllBytes($path, $png)
  return $png.Length -gt 50000
}

$apps = @(
  @{ n='material'; a='com.angnative.app.MainActivity' },
  @{ n='primeng'; a='com.angnative.app.MainActivity' },
  @{ n='ngzorro'; a='com.angnative.app.MainActivity' },
  @{ n='taiga'; a='com.angnative.app.MainActivity' },
  @{ n='spartan'; a='com.angnative.app.MainActivity' },
  @{ n='ecosystem-grid'; a='com.angnative.app.MainActivity' },
  @{ n='ecosystem-charts'; a='com.angnative.app.MainActivity' },
  @{ n='ecosystem-core'; a='com.angnative.app.EcosystemCoreActivity' },
  @{ n='ecosystem-richtext'; a='com.angnative.app.MainActivity' },
  @{ n='ecosystem-calendar'; a='com.angnative.app.MainActivity' }
)
foreach ($a in $apps) {
  $p = ($a.n -replace '^', 'com.angnative.demo.')
  & $adb -s $dev shell am force-stop $p 2>$null
  & $adb -s $dev shell am start -n "$p/$($a.a)" 2>$null | Out-Null
  Start-Sleep -Seconds 4
  $ok = Save-ValidScreenshot (Join-Path $outDir "$($a.n)-valid.png")
  Write-Output "$($a.n): screenshot=$ok"
}
Write-Output 'SCREENSHOTS DONE'
