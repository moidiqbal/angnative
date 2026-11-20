# Windows counterpart of build-android-native.sh — rebuild Rust host libs for Android ABIs.
$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
$sdk = 'C:\Android\Sdk'
$ndk = Join-Path $sdk 'ndk\27.2.12479018'
$bin = Join-Path $ndk 'toolchains\llvm\prebuilt\windows-x86_64\bin'
$ar = Join-Path $bin 'llvm-ar.exe'

$env:ANDROID_HOME = $sdk
$env:ANDROID_SDK_ROOT = $sdk
$env:ANDROID_NDK_HOME = $ndk
$env:PATH = "$bin;$env:PATH"

$targets = @(
  @{ triple = 'aarch64-linux-android';    abi = 'arm64-v8a';     clang = 'aarch64-linux-android24-clang.cmd' },
  @{ triple = 'armv7-linux-androideabi';  abi = 'armeabi-v7a';   clang = 'armv7a-linux-androideabi24-clang.cmd' },
  @{ triple = 'x86_64-linux-android';     abi = 'x86_64';        clang = 'x86_64-linux-android24-clang.cmd' }
)

Set-Location $repo
foreach ($t in $targets) {
  $upper = $t.triple.ToUpperInvariant().Replace('-', '_')
  [Environment]::SetEnvironmentVariable("CARGO_TARGET_${upper}_LINKER", (Join-Path $bin $t.clang), 'Process')
  [Environment]::SetEnvironmentVariable("CARGO_TARGET_${upper}_AR", $ar, 'Process')

  Write-Output "=== Building $($t.triple) ($($t.abi)) ==="
  cargo build --release --target $t.triple --package angnative-host-ffi
  if ($LASTEXITCODE -ne 0) { throw "cargo build failed for $($t.triple)" }

  $src = Join-Path $repo "target\$($t.triple)\release\libangnative_host_ffi.a"
  $dest = Join-Path $repo "platforms\android\app\src\main\jniLibs\$($t.abi)"
  New-Item -ItemType Directory -Force $dest | Out-Null
  Copy-Item $src (Join-Path $dest 'libangnative_host_ffi.a') -Force
  Write-Output "  copied -> $dest"
}
Write-Output 'Android Rust libraries rebuilt.'
