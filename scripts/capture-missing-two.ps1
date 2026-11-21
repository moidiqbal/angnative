# Capture the 2 missing showcase screenshots with a STRICT foreground guard.
# Never save a frame unless mCurrentFocus belongs to the target app.
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Drawing
$adb='C:\Android\Sdk\platform-tools\adb.exe'; $dev='b6640c8f'
$repo=Split-Path -Parent $PSScriptRoot; $out=Join-Path $repo 'docs\benchmarks\physical'
$apps=@(
  @{n='ecosystem-richtext';p='com.angnative.demo.ecosystem.richtext';a='com.angnative.app.MainActivity'},
  @{n='ecosystem-calendar';p='com.angnative.demo.ecosystem.calendar';a='com.angnative.app.MainActivity'}
)
foreach($x in $apps){
  # Clear notification shade aggressively before launch.
  & $adb -s $dev shell input keyevent 111 2>$null   # ESC / collapse shade
  & $adb -s $dev shell cmd statusbar collapse 2>$null
  Start-Sleep -Milliseconds 500
  & $adb -s $dev shell am force-stop $x.p 2>$null
  & $adb -s $dev shell am start -n "$($x.p)/$($x.a)" 2>$null | Out-Null

  $focused=$false
  for($try=0;$try -lt 12;$try++){
    Start-Sleep -Seconds 1
    $w=(& $adb -s $dev shell dumpsys window 2>&1 | Select-String 'mCurrentFocus' | Select-Object -First 1 | Out-String)
    if($w -match $x.p -and $w -notmatch 'NotificationShade'){ $focused=$true; break }
    # dismiss any shade that reappeared
    & $adb -s $dev shell input keyevent 111 2>$null
    & $adb -s $dev shell cmd statusbar collapse 2>$null
  }
  if(-not $focused){ "$($x.n): SKIPPED (never foreground)"; continue }

  $raw=Join-Path $env:TEMP 'cm2.raw'; & $adb -s $dev exec-out screencap -p > $raw
  $b=[System.IO.File]::ReadAllBytes($raw); $s=-1
  for($i=0;$i -lt [Math]::Min(3000,$b.Length-8);$i++){ if($b[$i] -eq 0x89 -and $b[$i+1] -eq 0x50){$s=$i;break} }
  if($s -lt 0){ "$($x.n): NO PNG"; continue }
  $full=Join-Path $env:TEMP 'cm2full.png'; [System.IO.File]::WriteAllBytes($full,$b[$s..($b.Length-1)])
  $img=[System.Drawing.Image]::FromFile($full); $w=$img.Width; $h=$img.Height-140-130
  $bmp=New-Object System.Drawing.Bitmap($w,$h); $g=[System.Drawing.Graphics]::FromImage($bmp)
  $g.DrawImage($img,(New-Object System.Drawing.Rectangle(0,0,$w,$h)),(New-Object System.Drawing.Rectangle(0,140,$w,$h)),[System.Drawing.GraphicsUnit]::Pixel)
  $dest=Join-Path $out "$($x.n).png"; $bmp.Save($dest,[System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose();$bmp.Dispose();$img.Dispose()
  "$($x.n): saved with strict focus guard"
}
