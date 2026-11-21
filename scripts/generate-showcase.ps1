# Clone the verified native Material showcase into a sibling showcase.
# Zero WebView: every clone uses the same JNI -> NativeViewFactory -> Android views path.
param(
  [Parameter(Mandatory=$true)][string]$Name,      # e.g. primeng
  [Parameter(Mandatory=$true)][string]$PackageId, # e.g. com.angnative.demo.primeng
  [Parameter(Mandatory=$true)][string]$Label      # e.g. "PrimeNG"
)
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$template = Join-Path $repo 'showcases\material'
$target = Join-Path $repo "showcases\$Name"

if (-not (Test-Path $template)) { throw "Template missing: $template" }
if (Test-Path $target) { Remove-Item $target -Recurse -Force }

Copy-Item -Path $template -Destination $target -Recurse
Get-ChildItem $target -Recurse -Directory | Where-Object { $_.Name -in @('build','.cxx','.gradle','.kotlin') } | Remove-Item -Recurse -Force
Remove-Item (Join-Path $target 'local.properties') -ErrorAction SilentlyContinue

# Package id in Gradle + Kotlin namespace declarations
$gradle = Join-Path $target 'app\build.gradle.kts'
(Get-Content $gradle) -replace 'com\.angnative\.demo\.material', $PackageId | Set-Content $gradle

# Branding strings
$strings = Join-Path $target 'app\src\main\res\values\strings.xml'
(Get-Content $strings) -replace 'AngNative Material Showcase \(Native\)', "AngNative $Label Showcase (Native)" | Set-Content $strings

# Kotlin source: namespace path + labels. Move java package folder if needed.
$srcRoot = Join-Path $target 'app\src\main\java\com\angnative\app'
foreach ($f in Get-ChildItem $srcRoot -Filter *.kt) {
  $c = Get-Content $f.FullName -Raw
  $c = $c -replace 'ANGNATIVE • MATERIAL SHOWCASE \(NATIVE REIMPLEMENTATION\)', "ANGNATIVE • $($Label.ToUpper()) SHOWCASE (NATIVE REIMPLEMENTATION)"
  $c = $c -replace 'Material 3 — Native Showcase', "$Label — Native Showcase"
  Set-Content $f.FullName $c
}

Write-Output "Generated native showcase: $Name ($PackageId) at $target"
