# Build every native showcase APK and stage artifacts under dist/showcases/.
# Zero WebView: all use the JNI -> NativeViewFactory -> Android views path.
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$sdk = 'C:\Android\Sdk'
$env:ANDROID_HOME = $sdk
$env:ANDROID_SDK_ROOT = $sdk
$env:JAVA_HOME = 'C:\Program Files\Java\jdk-25.0.5.1'
$env:PATH = "$env:JAVA_HOME\bin;$env:PATH"

$showcases = @('material','primeng','ngzorro','taiga','spartan')
foreach ($name in $showcases) {
  $dir = Join-Path $repo "showcases\$name"
  if (-not (Test-Path $dir)) { Write-Output "SKIP $name (missing)"; continue }
  Set-Content (Join-Path $dir 'local.properties') "sdk.dir=$($sdk -replace '\\','/')"

  Write-Output "=== BUILD $name ==="
  Push-Location $dir
  & .\gradlew.bat assembleDebug --no-daemon 2>&1 | Select-String -Pattern 'BUILD SUCCESSFUL|BUILD FAILED|error:' | Select-Object -Last 3
  $ok = $LASTEXITCODE -eq 0
  Pop-Location
  if (-not $ok) { Write-Output "FAILED: $name"; continue }

  $apk = Join-Path $dir 'app\build\outputs\apk\debug\app-debug.apk'
  $out = Join-Path $repo "dist\showcases\$name"
  New-Item -ItemType Directory -Force $out | Out-Null
  Copy-Item $apk (Join-Path $out 'app-debug.apk') -Force

  # source.zip (exclude build outputs)
  $tmp = Join-Path $env:TEMP "angnative-showcase-$name"
  if (Test-Path $tmp) { Remove-Item $tmp -Recurse -Force }
  New-Item -ItemType Directory -Force $tmp | Out-Null
  robocopy $dir $tmp /E /XD build .cxx .gradle .kotlin /XF local.properties /NFL /NDL /NJH /NJS /NC /NS | Out-Null
  Compress-Archive -Path (Join-Path $tmp '*') -DestinationPath (Join-Path $out 'source.zip') -Force

  $size = [math]::Round((Get-Item $apk).Length/1MB, 2)
  Write-Output "OK $name — APK ${size} MB"
}
Write-Output 'ALL SHOWCASE BUILDS DONE'
