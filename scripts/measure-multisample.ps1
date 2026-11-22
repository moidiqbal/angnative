# Emulator multi-sample frame pacing + CPU evidence for the native showcases.
# Usage: pwsh -File scripts/measure-multisample.ps1 -SampleCount 3
param(
  [ValidateRange(1, 10)][int]$SampleCount = 3
)

$ErrorActionPreference = 'Stop'
$adb = 'C:\Android\Sdk\platform-tools\adb.exe'

$devices = (& $adb devices 2>$null | Out-String)
if ($devices -notmatch 'emulator-\d+\s+device') {
  Write-Output 'SKIP multi-sample benchmark — no online emulator'
  exit 0
}

$apps = [ordered]@{
  'ecosystem-grid'     = 'com.angnative.demo.ecosystem.grid'
  'ecosystem-charts'   = 'com.angnative.demo.ecosystem.charts'
  'ecosystem-richtext' = 'com.angnative.demo.ecosystem.richtext'
  'ecosystem-calendar' = 'com.angnative.demo.ecosystem.calendar'
}

function Get-FirstMatch([string]$text, [string]$pattern) {
  $m = [regex]::Match($text, $pattern)
  if ($m.Success) { return [double]$m.Groups[1].Value }
  return -1
}

function Get-CpuPercent([string]$package) {
  $appPid = (& $adb shell pidof $package 2>$null | Out-String).Trim()
  if ($appPid -notmatch '^\d+$') { return -1 }
  $raw = (& $adb shell top -n 1 -p $appPid 2>$null | Out-String)
  $line = $raw -split "`n" | Where-Object { $_ -match "^\s*$appPid\s" } | Select-Object -First 1
  if (-not $line) { return -1 }
  $fields = @($line -split '\s+' | Where-Object { $_ -ne '' })
  if ($fields.Count -lt 9) { return -1 }
  $value = 0.0
  if ([double]::TryParse($fields[8], [ref]$value)) { return $value }
  return -1
}

$results = @()
foreach ($name in $apps.Keys) {
  $package = $apps[$name]
  $samples = @()

  for ($i = 1; $i -le $SampleCount; $i++) {
    & $adb shell am force-stop $package 2>$null | Out-Null
    & $adb shell dumpsys gfxinfo $package reset 2>$null | Out-Null
    Start-Sleep -Milliseconds 400
    & $adb shell am start -n "$package/com.angnative.app.MainActivity" 2>$null | Out-Null
    Start-Sleep -Seconds 3

    $cpu = @()
    for ($swipe = 0; $swipe -lt 6; $swipe++) {
      & $adb shell input swipe 540 1700 540 600 250 2>$null | Out-Null
      $cpu += Get-CpuPercent $package
      Start-Sleep -Milliseconds 200
    }
    Start-Sleep -Seconds 1

    $stats = (& $adb shell dumpsys gfxinfo $package framestats 2>$null | Out-String)
    $sample = [ordered]@{
      sample         = $i
      frames         = [int](Get-FirstMatch $stats 'Total frames rendered:\s*(\d+)')
      janky_percent  = Get-FirstMatch $stats 'Janky frames:\s*\d+\s*\(([\d.]+)%\)'
      p50_ms         = Get-FirstMatch $stats '50th percentile:\s*(\d+)ms'
      p95_ms         = Get-FirstMatch $stats '95th percentile:\s*(\d+)ms'
      cpu_samples    = @($cpu | Where-Object { $_ -ge 0 })
    }
    $validCpu = @($sample.cpu_samples)
    if ($validCpu.Count -gt 0) {
      $sample.cpu_percent_avg = [math]::Round((($validCpu | Measure-Object -Average).Average), 2)
      $sample.cpu_percent_max = [math]::Round((($validCpu | Measure-Object -Maximum).Maximum), 2)
    } else {
      $sample.cpu_percent_avg = -1
      $sample.cpu_percent_max = -1
    }
    $samples += $sample
  }

  $valid = @($samples | Where-Object { $_.frames -gt 0 })
  if ($valid.Count -eq 0) { continue }
  $results += [ordered]@{
    name            = $name
    package         = $package
    samples         = $samples
    avg_frames      = [math]::Round((($valid | ForEach-Object { $_.frames } | Measure-Object -Average).Average), 1)
    avg_janky_pct   = [math]::Round((($valid | ForEach-Object { $_.janky_percent } | Measure-Object -Average).Average), 2)
    avg_p50_ms      = [math]::Round((($valid | ForEach-Object { $_.p50_ms } | Measure-Object -Average).Average), 1)
    avg_p95_ms      = [math]::Round((($valid | ForEach-Object { $_.p95_ms } | Measure-Object -Average).Average), 1)
    avg_cpu_pct     = [math]::Round((($valid | ForEach-Object { $_.cpu_percent_avg } | Where-Object { $_ -ge 0 } | Measure-Object -Average).Average), 2)
    max_cpu_pct     = [math]::Round((($valid | ForEach-Object { $_.cpu_percent_max } | Where-Object { $_ -ge 0 } | Measure-Object -Maximum).Maximum), 2)
  }
}

$report = [ordered]@{
  measured_at = (Get-Date).ToString('o')
  device      = ((& $adb shell getprop ro.product.model 2>$null | Out-String).Trim() + ' / Android ' + (& $adb shell getprop ro.build.version.release 2>$null | Out-String).Trim())
  method      = "dumpsys gfxinfo framestats + top CPU sampled during each 6-swipe burst; $SampleCount bursts per app, foreground app confirmed by launch"
  note        = 'Emulator SwiftShader software rendering. Multi-sample averages reduce single-burst noise but are NOT physical-device GPU figures. CPU values are instantaneous top samples taken during the burst and are often 0.0; they are not sustained CPU utilization.'
  results     = $results
}

$outDir = Join-Path (Split-Path -Parent $PSScriptRoot) 'docs\benchmarks'
New-Item -ItemType Directory -Force $outDir | Out-Null
$file = Join-Path $outDir 'emulator-multisample.json'
Set-Content $file ($report | ConvertTo-Json -Depth 6)
Write-Output ($report | ConvertTo-Json -Depth 6)
Write-Output "saved: $file"
