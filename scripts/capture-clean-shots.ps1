# Clean showcase screenshots: crop status bar (personal notifications) from top.
# OnePlus Open foldable: adb screencap prepends a warning to stdout -> strip PNG magic.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$adb = 'C:\Android\Sdk\platform-tools\adb.exe'
$dev = 'b6640c8f'
$repo = Split-Path -Parent $PSScriptRoot
$outDir = Join-Path $repo 'docs\benchmarks\physical'
New-Item -ItemType Directory -Force $outDir | Out-Null

function Get-CleanShot([string]$path, [int]$cropTop = 140, [int]$cropBottom = 130) {
  $raw = Join-Path $env:TEMP 'angnative-clean.raw'
  & $adb -s $dev exec-out screencap -p > $raw
  $b = [System.IO.File]::ReadAllBytes($raw)
  $start = -1
  for ($i=0; $i -lt [Math]::Min(3000, $b.Length-8); $i++) {
    if ($b[$i] -eq 0x89 -and $b[$i+1] -eq 0x50 -and $b[$i+2] -eq 0x4E -and $b[$i+3] -eq 0x47) { $start=$i; break }
  }
  if ($start -lt 0) { return $false }
  $tmpPng = Join-Path $env:TEMP 'angnative-clean-full.png'
  [System.IO.File]::WriteAllBytes($tmpPng, $b[$start..($b.Length-1)])
  try {
    $img = [System.Drawing.Image]::FromFile($tmpPng)
    $w = $img.Width; $h = $img.Height - $cropTop - $cropBottom
    $bmp = New-Object System.Drawing.Bitmap($w, $h)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.DrawImage($img, (New-Object System.Drawing.Rectangle(0,0,$w,$h)),
                 (New-Object System.Drawing.Rectangle(0,$cropTop,$w,$h)),
                 [System.Drawing.GraphicsUnit]::Pixel)
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose(); $img.Dispose()
    return $true
  } catch { return $false }
}

$apps = @(
  @{ n='material'; p='com.angnative.demo.material'; a='com.angnative.app.MainActivity' },
  @{ n='primeng'; p='com.angnative.demo.primeng'; a='com.angnative.app.MainActivity' },
  @{ n='ngzorro'; p='com.angnative.demo.ngzorro'; a='com.angnative.app.MainActivity' },
  @{ n='taiga'; p='com.angnative.demo.taiga'; a='com.angnative.app.MainActivity' },
  @{ n='spartan'; p='com.angnative.demo.spartan'; a='com.angnative.app.MainActivity' },
  @{ n='ecosystem-grid'; p='com.angnative.demo.ecosystem.grid'; a='com.angnative.app.MainActivity' },
  @{ n='ecosystem-charts'; p='com.angnative.demo.ecosystem.charts'; a='com.angnative.app.MainActivity' },
  @{ n='ecosystem-core'; p='com.angnative.demo.ecosystem.core'; a='com.angnative.app.EcosystemCoreActivity' },
  @{ n='ecosystem-richtext'; p='com.angnative.demo.ecosystem.richtext'; a='com.angnative.app.MainActivity' },
  @{ n='ecosystem-calendar'; p='com.angnative.demo.ecosystem.calendar'; a='com.angnative.app.MainActivity' }
)
foreach ($a in $apps) {
  $p = $a.p
  & $adb -s $dev shell am force-stop $p 2>$null
  & $adb -s $dev shell am start -n "$p/$($a.a)" 2>$null | Out-Null
  Start-Sleep -Seconds 4
  $ok = Get-CleanShot (Join-Path $outDir "$($a.n).png")
  Write-Output "$($a.n): clean_crop=$ok"
}
Write-Output 'CLEAN SCREENSHOTS DONE'
