# Sustained-load Android benchmark for all AngNative showcases.
# Usage:
#   pwsh -File scripts/measure-sustained.ps1 -SelfTest
#   pwsh -File scripts/measure-sustained.ps1 -Device emulator-5554
#   pwsh -File scripts/measure-sustained.ps1 -Device b6640c8f -DurationSeconds 60
param(
  [string]$Adb = 'C:\Android\Sdk\platform-tools\adb.exe',
  [string]$Device = 'emulator-5554',
  [ValidateRange(1, 10)][int]$Samples = 3,
  [ValidateRange(10, 120)][int]$DurationSeconds = 30,
  [ValidateRange(1, 10)][int]$StartReps = 5,
  [ValidateRange(200, 5000)][int]$CpuIntervalMs = 1000,
  [ValidateRange(1000, 30000)][int]$MemIntervalMs = 5000,
  [ValidateRange(100, 1000)][int]$SwipeDurationMs = 250,
  [string[]]$Apps,
  [switch]$NoBoot,
  [switch]$SelfTest,
  [string]$Output
)

$ErrorActionPreference = 'Stop'
$script:AdbPath = $Adb
$script:Serial = $Device

# Package -> exact launcher activity. ecosystem-core is the only non-MainActivity.
# MarketPulse reuses the generic shell package but is staged in dist/showcases,
# so it carries an explicit APK path (the other 10 apps use the device-testing
# <name>-<package>.apk convention via the fallback below).
$AppMap = [ordered]@{
  'material'            = [pscustomobject]@{ Package = 'com.angnative.demo.material'; Activity = 'com.angnative.app.MainActivity' }
  'primeng'             = [pscustomobject]@{ Package = 'com.angnative.demo.primeng'; Activity = 'com.angnative.app.MainActivity' }
  'ngzorro'             = [pscustomobject]@{ Package = 'com.angnative.demo.ngzorro'; Activity = 'com.angnative.app.MainActivity' }
  'taiga'               = [pscustomobject]@{ Package = 'com.angnative.demo.taiga'; Activity = 'com.angnative.app.MainActivity' }
  'spartan'             = [pscustomobject]@{ Package = 'com.angnative.demo.spartan'; Activity = 'com.angnative.app.MainActivity' }
  'ecosystem-grid'      = [pscustomobject]@{ Package = 'com.angnative.demo.ecosystem.grid'; Activity = 'com.angnative.app.MainActivity' }
  'ecosystem-charts'    = [pscustomobject]@{ Package = 'com.angnative.demo.ecosystem.charts'; Activity = 'com.angnative.app.MainActivity' }
  'ecosystem-core'      = [pscustomobject]@{ Package = 'com.angnative.demo.ecosystem.core'; Activity = 'com.angnative.app.EcosystemCoreActivity' }
  'ecosystem-richtext'  = [pscustomobject]@{ Package = 'com.angnative.demo.ecosystem.richtext'; Activity = 'com.angnative.app.MainActivity' }
  'ecosystem-calendar'  = [pscustomobject]@{ Package = 'com.angnative.demo.ecosystem.calendar'; Activity = 'com.angnative.app.MainActivity' }
  'market-pulse'        = [pscustomobject]@{ Package = 'com.angnative.app'; Activity = 'com.angnative.app.MainActivity'; Apk = 'dist\showcases\market-pulse\app-debug.apk' }
}

function Match-Number([string]$Text, [string]$Pattern) {
  if ([string]::IsNullOrEmpty($Text)) { return $null }
  $m = [regex]::Match($Text, $Pattern)
  if (-not $m.Success) { return $null }
  $value = 0.0
  if ([double]::TryParse($m.Groups[1].Value, [ref]$value)) { return $value }
  return $null
}

function Match-Int([string]$Text, [string]$Pattern) {
  $v = Match-Number $Text $Pattern
  if ($null -eq $v) { return $null }
  return [int]$v
}

# Android 14 sometimes omits TotalTime for warm/HOT launches. WaitTime is the
# documented fallback and is labelled as such in the retained JSON.
function Parse-StartTiming([string]$Text) {
  $total = Match-Int $Text 'TotalTime:\s*(\d+)(?:ms)?'
  $source = 'TotalTime'
  if ($null -eq $total) {
    $total = Match-Int $Text 'WaitTime:\s*(\d+)(?:ms)?'
    if ($null -ne $total) { $source = 'WaitTime (TotalTime omitted by am start -W)' }
    else { $source = 'unavailable' }
  }
  return [ordered]@{ total_ms = $total; source = $source }
}

function Round-Null([object]$Value, [int]$Digits = 2) {
  if ($null -eq $Value) { return $null }
  return [math]::Round([double]$Value, $Digits)
}

function Parse-Gfxinfo([string]$Text) {
  return [ordered]@{
    frames      = Match-Int $Text 'Total frames rendered:\s*(\d+)'
    janky_pct   = Match-Number $Text 'Janky frames:\s*\d+\s*\(([\d.]+)%\)'
    p50_ms      = Match-Int $Text '50th percentile:\s*(\d+)ms'
    p95_ms      = Match-Int $Text '95th percentile:\s*(\d+)ms'
    p99_ms      = Match-Int $Text '99th percentile:\s*(\d+)ms'
  }
}

function Normalize-FrameMetrics([object]$Stats) {
  $meaningful = ($null -ne $Stats.frames -and $Stats.frames -gt 0)
  return [ordered]@{
    meaningful = $meaningful
    frames      = $Stats.frames
    janky_pct   = if ($meaningful) { $Stats.janky_pct } else { $null }
    p50_ms      = if ($meaningful) { $Stats.p50_ms } else { $null }
    p95_ms      = if ($meaningful) { $Stats.p95_ms } else { $null }
    p99_ms      = if ($meaningful) { $Stats.p99_ms } else { $null }
  }
}

function Parse-Meminfo([string]$Text) {
  return [ordered]@{
    pss_kb    = Match-Int $Text '(?m)^\s*TOTAL PSS:\s*(\d+)'
    webviews  = Match-Int $Text '(?m)^\s*WebViews:\s*(\d+)'
    views     = Match-Int $Text '(?m)^\s*Views:\s*(\d+)'
  }
}

# /proc/<pid>/stat field 14 is utime, field 15 is stime. After the last ')'
# the first token is field 3, so utime/stime are indexes 11 and 12.
function Parse-ProcStatSeconds([string]$Text, [int]$ClockTicks) {
  if ([string]::IsNullOrEmpty($Text)) { return $null }
  $idx = $Text.LastIndexOf(')')
  if ($idx -lt 0) { return $null }
  $fields = @($Text.Substring($idx + 1).Trim() -split '\s+' | Where-Object { $_ -ne '' })
  if ($fields.Count -lt 13 -or $ClockTicks -lt 1) { return $null }
  $utime = 0.0
  $stime = 0.0
  if (-not [double]::TryParse($fields[11], [ref]$utime)) { return $null }
  if (-not [double]::TryParse($fields[12], [ref]$stime)) { return $null }
  return (($utime + $stime) / [double]$ClockTicks)
}

function Parse-TopPercent([string]$Text, [string]$AppPid) {
  if ([string]::IsNullOrEmpty($Text)) { return $null }
  $line = @($Text -split "`r?`n" | Where-Object { $_ -match "^\s*$AppPid\s" } | Select-Object -First 1)
  if ($line.Count -eq 0) { return $null }
  $fields = @($line[0].Trim() -split '\s+' | Where-Object { $_ -ne '' })
  if ($fields.Count -lt 9) { return $null }
  $value = 0.0
  if ([double]::TryParse($fields[8], [ref]$value)) { return $value }
  return $null
}

function Get-Percentile([double[]]$Values, [double]$Percent) {
  if ($null -eq $Values -or $Values.Count -eq 0) { return $null }
  $sorted = @($Values | Sort-Object)
  $rank = [int][math]::Ceiling(($Percent / 100.0) * $sorted.Count)
  if ($rank -lt 1) { $rank = 1 }
  if ($rank -gt $sorted.Count) { $rank = $sorted.Count }
  return [double]$sorted[$rank - 1]
}

function New-NumericStats([array]$Values) {
  $clean = @($Values | Where-Object { $null -ne $_ } | ForEach-Object { [double]$_ })
  if ($clean.Count -eq 0) {
    return [ordered]@{ n = 0; mean = $null; median = $null; min = $null; max = $null; p95 = $null }
  }
  return [ordered]@{
    n      = $clean.Count
    mean   = Round-Null (($clean | Measure-Object -Average).Average) 2
    median = Round-Null (Get-Percentile $clean 50) 2
    min    = Round-Null (($clean | Measure-Object -Minimum).Minimum) 2
    max    = Round-Null (($clean | Measure-Object -Maximum).Maximum) 2
    p95    = Round-Null (Get-Percentile $clean 95) 2
  }
}

function Get-LinearSlope([array]$Points) {
  $pairs = @($Points | Where-Object { $null -ne $_.t_s -and $null -ne $_.pss_kb })
  if ($pairs.Count -lt 2) { return $null }
  $meanX = ($pairs | ForEach-Object { [double]$_.t_s } | Measure-Object -Average).Average
  $meanY = ($pairs | ForEach-Object { [double]$_.pss_kb } | Measure-Object -Average).Average
  $num = 0.0
  $den = 0.0
  foreach ($p in $pairs) {
    $dx = [double]$p.t_s - $meanX
    $num += $dx * ([double]$p.pss_kb - $meanY)
    $den += $dx * $dx
  }
  if ($den -eq 0) { return $null }
  return ($num / $den)
}

function New-StartGroup([array]$Items) {
  $process = @($Items | Where-Object { $_.valid -and $null -ne $_.process_ms } | ForEach-Object { [double]$_.process_ms })
  $total = @($Items | Where-Object { $_.valid -and $null -ne $_.total_time_ms } | ForEach-Object { [double]$_.total_time_ms })
  return [ordered]@{
    samples       = @($Items)
    process_ms    = New-NumericStats $process
    total_time_ms = New-NumericStats $total
  }
}

function Assert-SelfTest([string]$Name, [object]$Expected, [object]$Actual) {
  if ("$Expected" -ne "$Actual") { throw "SELFTEST FAIL $Name : expected=$Expected actual=$Actual" }
  Write-Output "SELFTEST PASS: $Name"
}

if ($SelfTest) {
  $gfxText = @'
Total frames rendered: 123
Janky frames: 12 (9.76%)
50th percentile: 16ms
95th percentile: 33ms
99th percentile: 48ms
'@
  $gfx = Parse-Gfxinfo $gfxText
  Assert-SelfTest 'gfxinfo frames' 123 $gfx.frames
  Assert-SelfTest 'gfxinfo janky' 9.76 $gfx.janky_pct
  Assert-SelfTest 'gfxinfo p99' 48 $gfx.p99_ms

  $memText = @'
TOTAL PSS: 12345
WebViews: 0
Views: 456
'@
  $mem = Parse-Meminfo $memText
  Assert-SelfTest 'meminfo pss' 12345 $mem.pss_kb
  Assert-SelfTest 'meminfo views' 456 $mem.views

  $statText = '1234 (com.angnative.demo) S 1 2 3 4 5 6 7 8 9 10 500 250 0'
  Assert-SelfTest 'procstat seconds' 7.5 (Parse-ProcStatSeconds $statText 100)

  $topText = "  PID PR IO PCY UID NI TIME+ %CPU`n  1234 0 0 0 0 0 0 0 12.5 0"
  Assert-SelfTest 'top percent' 12.5 (Parse-TopPercent $topText '1234')
  Assert-SelfTest 'start TotalTime' 345 (Match-Int 'TotalTime: 345ms' 'TotalTime:\s*(\d+)(?:ms)?')
  $timingFallback = Parse-StartTiming "Status: ok`nWaitTime: 130`nComplete"
  Assert-SelfTest 'start WaitTime fallback' 130 $timingFallback.total_ms
  Assert-SelfTest 'start timing source label' 'WaitTime (TotalTime omitted by am start -W)' $timingFallback.source
  $zeroFrames = Normalize-FrameMetrics (Parse-Gfxinfo "Total frames rendered: 0`n50th percentile: 4950ms`n95th percentile: 4950ms")
  Assert-SelfTest 'zero-frame p50 is null' $null $zeroFrames.p50_ms
  Assert-SelfTest 'zero-frame flag' $false $zeroFrames.meaningful

  $nums = @(100, 200, 300, 400, 500)
  Assert-SelfTest 'stats mean' 300 (New-NumericStats $nums).mean
  Assert-SelfTest 'stats median' 300 (New-NumericStats $nums).median
  Assert-SelfTest 'stats p95' 500 (New-NumericStats $nums).p95

  $points = @([pscustomobject]@{ t_s = 0; pss_kb = 100 }, [pscustomobject]@{ t_s = 10; pss_kb = 200 })
  Assert-SelfTest 'memory slope' 10 (Get-LinearSlope $points)
  Assert-SelfTest 'app map count' 11 $AppMap.Count
  Assert-SelfTest 'core activity' 'com.angnative.app.EcosystemCoreActivity' $AppMap['ecosystem-core'].Activity
  Assert-SelfTest 'market-pulse package' 'com.angnative.app' $AppMap['market-pulse'].Package
  Assert-SelfTest 'market-pulse activity' 'com.angnative.app.MainActivity' $AppMap['market-pulse'].Activity
  Assert-SelfTest 'market-pulse apk' 'dist\showcases\market-pulse\app-debug.apk' $AppMap['market-pulse'].Apk
  Write-Output 'SELFTEST OK'
  exit 0
}

function Invoke-AdbShell([string[]]$AdbArgs) {
  $raw = & $script:AdbPath -s $script:Serial @AdbArgs 2>&1 | Out-String
  return $raw
}

function Get-AppPid([string]$Package) {
  $raw = Invoke-AdbShell @('shell', 'pidof', $Package)
  $v = ($raw | Out-String).Trim()
  if ($v -match '^\d+$') { return $v }
  return $null
}

function Get-Foreground([string]$Package) {
  $activities = Invoke-AdbShell @('shell', 'dumpsys', 'activity', 'activities')
  $windows = Invoke-AdbShell @('shell', 'dumpsys', 'window')
  $resumed = @($activities -split "`r?`n" | Where-Object { $_ -match 'topResumedActivity' } | Select-Object -Last 1)
  $focus = @($windows -split "`r?`n" | Where-Object { $_ -match 'mCurrentFocus' } | Select-Object -Last 1)
  $resumedText = if ($resumed.Count -gt 0) { $resumed[0].Trim() } else { '' }
  $focusText = if ($focus.Count -gt 0) { $focus[0].Trim() } else { '' }
  $esc = [regex]::Escape($Package)
  return [ordered]@{
    ok      = ($resumedText -match $esc) -and ($focusText -match $esc)
    resumed = $resumedText
    focus   = $focusText
  }
}

function Ensure-Foreground([string]$Package) {
  $fg = Get-Foreground $Package
  if ($fg.ok) { return $fg }
  Invoke-AdbShell @('shell', 'input', 'keyevent', '224') | Out-Null
  Invoke-AdbShell @('shell', 'cmd', 'statusbar', 'collapse') | Out-Null
  Start-Sleep -Milliseconds 400
  return (Get-Foreground $Package)
}

function Get-DeviceInfo() {
  $modelRaw = (Invoke-AdbShell @('shell', 'getprop', 'ro.product.model')).Trim()
  $release = (Invoke-AdbShell @('shell', 'getprop', 'ro.build.version.release')).Trim()
  $sdkRaw = (Invoke-AdbShell @('shell', 'getprop', 'ro.build.version.sdk')).Trim()
  $hardware = (Invoke-AdbShell @('shell', 'getprop', 'ro.hardware')).Trim()
  $wm = (Invoke-AdbShell @('shell', 'wm', 'size')).Trim()
  $size = [regex]::Match($wm, '(\d+)x(\d+)')
  $width = if ($size.Success) { [int]$size.Groups[1].Value } else { 1080 }
  $height = if ($size.Success) { [int]$size.Groups[2].Value } else { 2400 }
  $clockRaw = (Invoke-AdbShell @('shell', 'getconf', 'CLK_TCK')).Trim()
  $clock = 100
  $clockParsed = 0
  if ([int]::TryParse($clockRaw, [ref]$clockParsed) -and $clockParsed -gt 0) { $clock = $clockParsed }
  $nprocRaw = (Invoke-AdbShell @('shell', 'nproc')).Trim()
  $nproc = 0
  [void][int]::TryParse($nprocRaw, [ref]$nproc)
  $renderer = if ($script:Serial -like 'emulator-*') {
    'SwiftShader software rendering (AVD started with -gpu swiftshader_indirect)'
  } else {
    "physical/unknown renderer (ro.hardware=$hardware)"
  }
  return [ordered]@{
    serial       = $script:Serial
    model        = $modelRaw
    android      = "Android $release (API $sdkRaw)"
    renderer     = $renderer
    wm_size      = $wm
    width        = $width
    height       = $height
    clock_ticks  = $clock
    nproc        = $nproc
  }
}

function Start-AppOnce([string]$Package, [string]$Activity, [string]$Mode, [int]$Rep) {
  $entry = [ordered]@{
    rep            = $Rep
    mode           = $Mode
    valid          = $false
    process_ms     = $null
    total_time_ms  = $null
    total_time_source = $null
    pid            = $null
    same_pid       = $null
    foreground_ok  = $false
    error          = $null
  }
  try {
    $priorPid = $null
    if ($Mode -eq 'cold') {
      Invoke-AdbShell @('shell', 'am', 'force-stop', $Package) | Out-Null
      Start-Sleep -Milliseconds 300
    } else {
      Invoke-AdbShell @('shell', 'input', 'keyevent', '3') | Out-Null
      Start-Sleep -Milliseconds 500
      $priorPid = Get-AppPid $Package
      if (-not $priorPid) { throw 'warm rep: process exited at HOME before relaunch' }
    }

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $startOut = Invoke-AdbShell @('shell', 'am', 'start', '-W', '-n', "$Package/$Activity")
    $timing = Parse-StartTiming $startOut
    $entry.total_time_ms = $timing.total_ms
    $entry.total_time_source = $timing.source
    $appPid = $null
    for ($i = 0; $i -lt 100; $i++) {
      Start-Sleep -Milliseconds 100
      $appPid = Get-AppPid $Package
      if ($appPid) { break }
    }
    $entry.process_ms = [int]$sw.ElapsedMilliseconds
    $entry.pid = $appPid
    if ($startOut -match 'Error type|FAILURE|Exception') { throw 'am start -W reported an error' }
    if (-not $appPid) { throw 'process did not appear within 10 s' }
    if ($Mode -eq 'warm') {
      $entry.same_pid = ($appPid -eq $priorPid)
      if (-not $entry.same_pid) { throw 'warm rep: PID changed (process was killed)' }
    }
    $fg = Ensure-Foreground $Package
    $entry.foreground_ok = $fg.ok
    if (-not $fg.ok) { throw 'foreground guard failed' }
    $entry.valid = ($entry.process_ms -gt 0)
  } catch {
    $entry.error = $_.Exception.Message
  }
  return $entry
}

function New-InvalidSample([int]$SampleNumber, [string]$Phase, [string]$Message) {
  return [ordered]@{
    sample          = $SampleNumber
    valid           = $false
    duration_s      = 0
    actual_swipes   = 0
    error           = [ordered]@{ phase = $Phase; message = $Message }
  }
}

function Run-SustainedSample([int]$SampleNumber, [string]$Package, [string]$Activity, $DeviceInfo) {
  $errors = [System.Collections.Generic.List[object]]::new()
  $sample = [ordered]@{
    sample          = $SampleNumber
    valid           = $false
    duration_s      = 0
    actual_swipes   = 0
    swipe_geometry  = $null
    foreground_start = $null
    foreground_end   = $null
    frames          = $null
    janky_pct       = $null
    p50_ms          = $null
    p95_ms          = $null
    p99_ms          = $null
    frame_metrics_meaningful = $false
    cpu             = $null
    memory          = $null
    errors          = @()
    low_frame_count = $false
  }
  try {
    Invoke-AdbShell @('shell', 'am', 'force-stop', $Package) | Out-Null
    Start-Sleep -Milliseconds 300
    Invoke-AdbShell @('shell', 'input', 'keyevent', '224') | Out-Null
    Invoke-AdbShell @('shell', 'am', 'start', '-n', "$Package/$Activity") | Out-Null
    Start-Sleep -Seconds 3

    $appPid = Get-AppPid $Package
    if (-not $appPid) { throw 'app PID not found after settle' }
    $fgStart = Ensure-Foreground $Package
    $sample.foreground_start = $fgStart
    if (-not $fgStart.ok) { throw 'foreground guard failed at sample start' }

    # Reset after settle so startup frames are excluded from the load interval.
    Invoke-AdbShell @('shell', 'dumpsys', 'gfxinfo', $Package, 'reset') | Out-Null

    $x = [int][math]::Round($DeviceInfo.width / 2)
    $yTop = [int][math]::Round($DeviceInfo.height * 0.78)
    $yBottom = [int][math]::Round($DeviceInfo.height * 0.28)
    $sample.swipe_geometry = [ordered]@{ x = $x; y_top = $yTop; y_bottom = $yBottom; duration_ms = $SwipeDurationMs }

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $clock = [int]$DeviceInfo.clock_ticks
    $cpuTimeline = [System.Collections.Generic.List[object]]::new()
    $memPoints = [System.Collections.Generic.List[object]]::new()

    $rawMem = Parse-Meminfo (Invoke-AdbShell @('shell', 'dumpsys', 'meminfo', $Package))
    $memPoints.Add([ordered]@{ t_s = 0.0; pss_kb = $rawMem.pss_kb; webviews = $rawMem.webviews; views = $rawMem.views })
    $prevCpu = Parse-ProcStatSeconds (Invoke-AdbShell @('shell', 'cat', "/proc/$appPid/stat")) $clock
    $prevAt = 0.0

    $nextCpuAt = 0
    $nextMemAt = 0
    $swipeCount = 0
    while ($sw.Elapsed.TotalSeconds -lt $DurationSeconds) {
      $down = ([int][math]::Floor($sw.Elapsed.TotalSeconds)) % 2 -eq 0
      if ($down) {
        Invoke-AdbShell @('shell', 'input', 'swipe', "$x", "$yTop", "$x", "$yBottom", "$SwipeDurationMs") | Out-Null
      } else {
        Invoke-AdbShell @('shell', 'input', 'swipe', "$x", "$yBottom", "$x", "$yTop", "$SwipeDurationMs") | Out-Null
      }
      $swipeCount++

      $elapsedMs = $sw.ElapsedMilliseconds
      if ($elapsedMs -ge $nextCpuAt) {
        $cum = Parse-ProcStatSeconds (Invoke-AdbShell @('shell', 'cat', "/proc/$appPid/stat")) $clock
        $topPct = Parse-TopPercent (Invoke-AdbShell @('shell', 'top', '-n', '1', '-p', $appPid)) $appPid
        $pct = $null
        if ($null -ne $cum -and $null -ne $prevCpu) {
          $deltaCpu = $cum - $prevCpu
          $deltaTime = ($elapsedMs / 1000.0) - $prevAt
          if ($deltaTime -gt 0 -and $deltaCpu -ge 0) { $pct = 100.0 * $deltaCpu / $deltaTime }
        }
        $cpuTimeline.Add([ordered]@{ t_s = [math]::Round($elapsedMs / 1000.0, 2); procstat_pct = $pct; top_pct = $topPct })
        if ($null -ne $cum) { $prevCpu = $cum; $prevAt = $elapsedMs / 1000.0 }
        $nextCpuAt = $elapsedMs + $CpuIntervalMs
      }
      if ($elapsedMs -ge $nextMemAt) {
        $m = Parse-Meminfo (Invoke-AdbShell @('shell', 'dumpsys', 'meminfo', $Package))
        $memPoints.Add([ordered]@{ t_s = [math]::Round($elapsedMs / 1000.0, 2); pss_kb = $m.pss_kb; webviews = $m.webviews; views = $m.views })
        $nextMemAt = $elapsedMs + $MemIntervalMs
      }
    }

    $endMs = $sw.ElapsedMilliseconds
    $cumEnd = Parse-ProcStatSeconds (Invoke-AdbShell @('shell', 'cat', "/proc/$appPid/stat")) $clock
    if ($null -ne $cumEnd -and $null -ne $prevCpu) {
      $deltaCpu = $cumEnd - $prevCpu
      $deltaTime = ($endMs / 1000.0) - $prevAt
      if ($deltaTime -gt 0 -and $deltaCpu -ge 0) {
        $cpuTimeline.Add([ordered]@{ t_s = [math]::Round($endMs / 1000.0, 2); procstat_pct = 100.0 * $deltaCpu / $deltaTime; top_pct = $null })
      }
    }
    $endMem = Parse-Meminfo (Invoke-AdbShell @('shell', 'dumpsys', 'meminfo', $Package))
    $memPoints.Add([ordered]@{ t_s = [math]::Round($endMs / 1000.0, 2); pss_kb = $endMem.pss_kb; webviews = $endMem.webviews; views = $endMem.views })

    $fgEnd = Ensure-Foreground $Package
    $sample.foreground_end = $fgEnd
    # gfxinfo reports a synthetic 4950 ms percentile when zero frames were
    # rendered. Keep the frame count but null the derived metrics.
    $stats = Normalize-FrameMetrics (Parse-Gfxinfo (Invoke-AdbShell @('shell', 'dumpsys', 'gfxinfo', $Package, 'framestats')))
    $sample.frames = $stats.frames
    $sample.frame_metrics_meaningful = $stats.meaningful
    $sample.janky_pct = $stats.janky_pct
    $sample.p50_ms = $stats.p50_ms
    $sample.p95_ms = $stats.p95_ms
    $sample.p99_ms = $stats.p99_ms
    $sample.duration_s = [math]::Round($endMs / 1000.0, 2)
    $sample.actual_swipes = $swipeCount
    $sample.low_frame_count = ($null -ne $stats.frames -and $stats.frames -lt 10)

    $procValues = @($cpuTimeline | Where-Object { $null -ne $_.procstat_pct } | ForEach-Object { [double]$_.procstat_pct })
    $topValues = @($cpuTimeline | Where-Object { $null -ne $_.top_pct } | ForEach-Object { [double]$_.top_pct })
    $sample.cpu = [ordered]@{
      nproc                      = [int]$DeviceInfo.nproc
      clock_ticks                = $clock
      timeline                   = @($cpuTimeline)
      procstat_pct_of_one_core   = @($procValues | ForEach-Object { Round-Null $_ 2 })
      procstat_avg_pct           = Round-Null (New-NumericStats $procValues).mean 2
      procstat_p95_pct           = Round-Null (New-NumericStats $procValues).p95 2
      procstat_max_pct           = Round-Null (New-NumericStats $procValues).max 2
      top_pct                    = @($topValues | ForEach-Object { Round-Null $_ 2 })
      top_avg_pct                = Round-Null (New-NumericStats $topValues).mean 2
      top_p95_pct                = Round-Null (New-NumericStats $topValues).p95 2
      top_max_pct                = Round-Null (New-NumericStats $topValues).max 2
    }

    $pssValues = @($memPoints | Where-Object { $null -ne $_.pss_kb } | ForEach-Object { [double]$_.pss_kb })
    $baseline = if ($pssValues.Count -gt 0) { [int]$pssValues[0] } else { $null }
    $endPss = if ($pssValues.Count -gt 0) { [int]$pssValues[-1] } else { $null }
    $peak = if ($pssValues.Count -gt 0) { [int]($pssValues | Measure-Object -Maximum).Maximum } else { $null }
    $sample.memory = [ordered]@{
      samples         = @($memPoints)
      baseline_pss_kb = $baseline
      end_pss_kb      = $endPss
      peak_pss_kb     = $peak
      delta_pss_kb    = if ($null -ne $baseline -and $null -ne $endPss) { $endPss - $baseline } else { $null }
      slope_kb_per_s  = Round-Null (Get-LinearSlope $memPoints) 2
    }

    if ($null -eq $stats.frames) { $errors.Add([ordered]@{ phase = 'gfxinfo'; message = 'no frame summary parsed' }) }
    if (-not $fgEnd.ok) { $errors.Add([ordered]@{ phase = 'foreground'; message = 'foreground guard failed at sample end' }) }
    $sample.errors = @($errors)
    $sample.valid = ($errors.Count -eq 0)
  } catch {
    $sample.valid = $false
    $errors.Add([ordered]@{ phase = 'sample'; message = $_.Exception.Message })
    $sample.errors = @($errors)
  }
  return $sample
}

function New-SampleAverages([array]$Samples) {
  $valid = @($Samples | Where-Object { $_.valid })
  $result = [ordered]@{
    sample_count        = $valid.Count
    insufficient_samples = ($valid.Count -lt 2)
    frames              = $null
    janky_pct           = $null
    p50_ms              = $null
    p95_ms              = $null
    cpu_procstat_pct    = $null
    cpu_top_pct         = $null
    mem_delta_kb        = $null
    mem_slope_kb_per_s  = $null
  }
  if ($valid.Count -eq 0) { return $result }
  $result.frames = Round-Null (New-NumericStats @($valid | ForEach-Object { $_.frames })).mean 1
  $result.janky_pct = Round-Null (New-NumericStats @($valid | ForEach-Object { $_.janky_pct })).mean 2
  $result.p50_ms = Round-Null (New-NumericStats @($valid | ForEach-Object { $_.p50_ms })).mean 1
  $result.p95_ms = Round-Null (New-NumericStats @($valid | ForEach-Object { $_.p95_ms })).mean 1
  $result.cpu_procstat_pct = Round-Null (New-NumericStats @($valid | ForEach-Object { if ($_.cpu) { $_.cpu.procstat_avg_pct } })).mean 2
  $result.cpu_top_pct = Round-Null (New-NumericStats @($valid | ForEach-Object { if ($_.cpu) { $_.cpu.top_avg_pct } })).mean 2
  $result.mem_delta_kb = Round-Null (New-NumericStats @($valid | ForEach-Object { if ($_.memory) { $_.memory.delta_pss_kb } })).mean 1
  $result.mem_slope_kb_per_s = Round-Null (New-NumericStats @($valid | ForEach-Object { if ($_.memory) { $_.memory.slope_kb_per_s } })).mean 2
  return $result
}

$repo = Split-Path -Parent $PSScriptRoot
$outDir = Join-Path $repo 'docs\benchmarks\sustained'
New-Item -ItemType Directory -Force $outDir | Out-Null

$selected = @()
if ($Apps -and $Apps.Count -gt 0) {
  foreach ($name in $Apps) {
    if (-not $AppMap.Contains($name)) { throw "Unknown app '$name'. Valid: $($AppMap.Keys -join ', ')" }
    $selected += $name
  }
} else {
  $selected = @($AppMap.Keys)
}

$deviceList = (& $Adb devices 2>&1 | Out-String)
if ($deviceList -notmatch "(?m)^\s*$([regex]::Escape($Device))\s+device\b") {
  if ($Device -like 'emulator-*' -and -not $NoBoot) {
    Write-Output "Device $Device offline — attempting one emulator boot ..."
    try {
      & (Join-Path $PSScriptRoot 'run-android-emulator.ps1')
      $deviceList = (& $Adb devices 2>&1 | Out-String)
    } catch {
      $bootError = $_.Exception.Message
    }
  }
  if ($deviceList -notmatch "(?m)^\s*$([regex]::Escape($Device))\s+device\b") {
    $reason = if ($bootError) { "emulator boot failed: $bootError" } else { "device $Device is not online" }
    $skipped = [ordered]@{
      schema_version = 1
      measured_at    = (Get-Date).ToString('o')
      status         = 'skipped'
      reason         = $reason
      device         = [ordered]@{ serial = $Device }
      method         = "$Samples x $DurationSeconds s alternating swipes, $StartReps cold + $StartReps warm starts"
      apps           = @()
      averages       = $null
      caveats        = @('No device run: no metrics were collected.')
      errors         = @([ordered]@{ phase = 'device'; message = $reason })
    }
    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $label = if ($Device -like 'emulator-*') { 'emulator' } else { ($Device -replace '[^A-Za-z0-9]', '-') }
    $file = if ($Output) { $Output } else { Join-Path $outDir "$label-$stamp.json" }
    Set-Content -Path $file -Value ($skipped | ConvertTo-Json -Depth 10) -Encoding utf8
    Write-Output "SKIP sustained benchmark — $reason"
    Write-Output "saved: $file"
    exit 0
  }
}

$deviceInfo = Get-DeviceInfo
Write-Output "Device: $($deviceInfo.model) / $($deviceInfo.android) / $($deviceInfo.serial)"

$method = "$Samples x $DurationSeconds s alternating swipes per app; $StartReps cold + $StartReps warm starts; gfxinfo reset after settle; CPU from /proc/<pid>/stat deltas every ${CpuIntervalMs} ms; meminfo every ${MemIntervalMs} ms"
$caveats = @(
  'SwiftShader emulator figures are not physical GPU figures; emulator and physical results are never averaged together.',
  'CPU procstat values are percent of one core, not device-wide utilization; top values are instantaneous and often 0.0.',
  'gfxinfo framestats summary is cumulative after the post-settle reset; detailed raw per-frame rows are ring-limited and are not retained.',
  'gfxinfo can report a synthetic 4950 ms percentile with zero rendered frames; those derived metrics are null and frame_metrics_meaningful=false.',
  'am start -W sometimes omits TotalTime on warm/HOT launches; WaitTime is used as a labelled fallback and start validity does not depend on either timing field.',
  'process_ms is wall time to am start -W plus pidof confirmation; it is not first-frame time and is not directly comparable to older non-blocking pidof-only samples.',
  'Low frame counts are flagged but retained; no app is dropped solely for scrolling less.'
)

$appResults = [System.Collections.Generic.List[object]]::new()
$allValidSamples = [System.Collections.Generic.List[object]]::new()
$runErrors = [System.Collections.Generic.List[object]]::new()

foreach ($name in $selected) {
  $spec = $AppMap[$name]
  # An explicit Apk override (currently only market-pulse) selects the staged
  # showcase APK instead of the dist/device-testing naming fallback.
  $apk = if ($spec.Apk) { Join-Path $repo $spec.Apk } else { Join-Path $repo "dist\device-testing\$name-$($spec.Package).apk" }
  $entry = [ordered]@{
    name      = $name
    package   = $spec.Package
    activity  = $spec.Activity
    apk_path  = $apk
    starts    = $null
    samples   = @()
    averages  = $null
    errors    = @()
  }
  $appErrors = [System.Collections.Generic.List[object]]::new()
  Write-Output "[$name] starts ..."
  try {
    $list = Invoke-AdbShell @('shell', 'pm', 'list', 'packages', $spec.Package)
    # Entries with an explicit APK path are reinstalled every run so a stale
    # com.angnative.app install cannot be measured in place of the staged build.
    if ($spec.Apk -or $list -notmatch [regex]::Escape($spec.Package)) {
      if (-not (Test-Path $apk)) { throw "APK not found: $apk" }
      $installOut = & $Adb -s $Device install -r $apk 2>&1 | Out-String
      if ($spec.Apk -and $installOut -notmatch 'Success') { throw "install failed: $apk`n$installOut" }
      $list = Invoke-AdbShell @('shell', 'pm', 'list', 'packages', $spec.Package)
      if ($list -notmatch [regex]::Escape($spec.Package)) { throw "install failed: $apk" }
    }

    $cold = @()
    for ($r = 1; $r -le $StartReps; $r++) { $cold += Start-AppOnce $spec.Package $spec.Activity 'cold' $r }
    $warm = @()
    for ($r = 1; $r -le $StartReps; $r++) { $warm += Start-AppOnce $spec.Package $spec.Activity 'warm' $r }
    $entry.starts = [ordered]@{ cold = New-StartGroup $cold; warm = New-StartGroup $warm }
    $badStarts = @($cold + $warm | Where-Object { -not $_.valid })
    if ($badStarts.Count -gt 0) {
      $appErrors.Add([ordered]@{ phase = 'starts'; message = "$($badStarts.Count) of $($StartReps * 2) start reps invalid" })
    }

    Write-Output "[$name] $Samples x $DurationSeconds s sustained ..."
    for ($s = 1; $s -le $Samples; $s++) {
      try {
        $sample = Run-SustainedSample $s $spec.Package $spec.Activity $deviceInfo
      } catch {
        $sample = New-InvalidSample $s 'sample' $_.Exception.Message
      }
      $entry.samples += $sample
      if ($sample.valid) { $allValidSamples.Add($sample) }
      Write-Output ("[{0}] sample {1}: valid={2} frames={3} swipes={4}" -f $name, $s, $sample.valid, $sample.frames, $sample.actual_swipes)
    }
    $entry.averages = New-SampleAverages $entry.samples
  } catch {
    $appErrors.Add([ordered]@{ phase = 'app'; message = $_.Exception.Message })
    $entry.averages = New-SampleAverages @()
  }
  $entry.errors = @($appErrors)
  foreach ($e in $appErrors) { $runErrors.Add($e) }
  $appResults.Add($entry)
}

$expected = $selected.Count * $Samples
$status = if ($allValidSamples.Count -eq 0) { 'skipped' } elseif ($allValidSamples.Count -lt $expected -or $runErrors.Count -gt 0) { 'partial' } else { 'ok' }

$report = [ordered]@{
  schema_version = 1
  measured_at    = (Get-Date).ToString('o')
  status         = $status
  device         = $deviceInfo
  method         = $method
  start_reps     = $StartReps
  sample_count   = $Samples
  duration_s     = $DurationSeconds
  apps           = @($appResults)
  averages       = New-SampleAverages @($allValidSamples)
  caveats        = $caveats
  errors         = @($runErrors)
}

$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$label = if ($Device -like 'emulator-*') { 'emulator' } else { ($Device -replace '[^A-Za-z0-9]', '-') }
$file = if ($Output) { $Output } else { Join-Path $outDir "$label-$stamp.json" }
$json = $report | ConvertTo-Json -Depth 14
Set-Content -Path $file -Value $json -Encoding utf8
Write-Output "status=$status valid_samples=$($allValidSamples.Count)/$expected"
Write-Output "saved: $file"
