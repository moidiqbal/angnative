# Windows counterpart of build-android-native.sh - rebuild the unified Rust
# library for Android ABIs via cargo + NDK.
#
# The runtime-bridge staticlib bundles host-ffi + host-engine + QuickJS + the
# runtime bridge into one archive. rquickjs-sys 0.14 has no Android bindings,
# so bindgen is enabled for Android with the NDK sysroot and VS LLVM libclang.
$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
$sdk = 'C:\Android\Sdk'
$ndk = Join-Path $sdk 'ndk\27.2.12479018'
$bin = Join-Path $ndk 'toolchains\llvm\prebuilt\windows-x86_64\bin'
$sysroot = (Join-Path $ndk 'toolchains\llvm\prebuilt\windows-x86_64\sysroot').Replace('\', '/')
$binFwd = $bin.Replace('\', '/')
$ar = Join-Path $bin 'llvm-ar.exe'
$libclang = 'C:/Program Files/Microsoft Visual Studio/18/Enterprise/VC/Tools/Llvm/x64/bin'
# rquickjs-sys needs GNU patch. Prefer one already on PATH; otherwise look in
# the usual GnuWin32 locations instead of hard-requiring one machine's layout.
$patchBin = $null
$patchCommand = Get-Command patch -CommandType Application -ErrorAction SilentlyContinue
if ($patchCommand -and $patchCommand.Source) {
  $patchBin = Split-Path -Parent $patchCommand.Source
} else {
  foreach ($candidate in @(
    'C:\Program Files (x86)\GnuWin32\bin',
    'C:\Program Files\GnuWin32\bin'
  )) {
    if (Test-Path -LiteralPath (Join-Path $candidate 'patch.exe')) {
      $patchBin = $candidate
      break
    }
  }
}
if (-not $patchBin) {
  throw "GNU patch not found. Install patch.exe and add it to PATH (or GnuWin32)."
}

$env:PATH = "$patchBin;C:\Program Files\Git\usr\bin;$bin;$env:PATH"
$env:ANDROID_HOME = $sdk
$env:ANDROID_SDK_ROOT = $sdk
$env:ANDROID_NDK_HOME = $ndk
$env:LIBCLANG_PATH = $libclang

# Where the unified archive is staged. The platform shell and every showcase
# copy the same JNI bridge sources and need the same library. Native .a files
# are deliberately NOT tracked (see .gitignore), so this script must stage
# all of them — including the showcases that still use the legacy
# libangnative_host_ffi.a filename in their CMakeLists.txt.
$destinations = @(
  (Join-Path $repo 'platforms\android\app\src\main\jniLibs')
)
$destinations += @(
  Get-ChildItem (Join-Path $repo 'showcases') -Directory | ForEach-Object {
    $jni = Join-Path $_.FullName 'app\src\main\jniLibs'
    if (Test-Path (Split-Path $jni -Parent)) { $jni }
  }
)

$targets = @(
  @{ triple = 'aarch64-linux-android';    abi = 'arm64-v8a';   clang = 'aarch64-linux-android24-clang.cmd';     incdir = 'aarch64-linux-android';   prefix = 'AARCH64_LINUX_ANDROID';    cc = 'CC_aarch64_linux_android' },
  @{ triple = 'armv7-linux-androideabi';  abi = 'armeabi-v7a'; clang = 'armv7a-linux-androideabi24-clang.cmd'; incdir = 'arm-linux-androideabi';   prefix = 'ARMV7_LINUX_ANDROIDEABI';  cc = 'CC_armv7_linux_androideabi' },
  @{ triple = 'x86_64-linux-android';     abi = 'x86_64';      clang = 'x86_64-linux-android24-clang.cmd';     incdir = 'x86_64-linux-android';    prefix = 'X86_64_LINUX_ANDROID';     cc = 'CC_x86_64_linux_android' }
)

Set-Location $repo
foreach ($t in $targets) {
  $clangPath = "$binFwd/$($t.clang)"
  $env:BINDGEN_EXTRA_CLANG_ARGS = "--sysroot=$sysroot -I$sysroot/usr/include -I$sysroot/usr/include/$($t.incdir) -D__ANDROID_API__=24"
  [Environment]::SetEnvironmentVariable($t.cc, $clangPath, 'Process')
  [Environment]::SetEnvironmentVariable("CARGO_TARGET_$($t.prefix)_LINKER", $clangPath, 'Process')
  [Environment]::SetEnvironmentVariable("CARGO_TARGET_$($t.prefix)_AR", $ar, 'Process')

  Write-Output "=== Building $($t.triple) ($($t.abi)) ==="
  cargo build --release --target $t.triple --package angnative-runtime-bridge
  if ($LASTEXITCODE -ne 0) { throw "cargo build failed for $($t.triple)" }

  $src = Join-Path $repo "target\$($t.triple)\release\libangnative_runtime_bridge.a"
  foreach ($dest in $destinations) {
    $abiDir = Join-Path $dest $t.abi
    New-Item -ItemType Directory -Force $abiDir | Out-Null
    Copy-Item $src (Join-Path $abiDir 'libangnative_runtime_bridge.a') -Force
    # Also replace the legacy-named archive. Some CMake configurations cache
    # the old filename; overwriting it guarantees the unified symbols are
    # linked regardless of which branch was selected.
    Copy-Item $src (Join-Path $abiDir 'libangnative_host_ffi.a') -Force
    Write-Output "  copied -> $abiDir"
  }
}
Write-Output 'Android Rust libraries rebuilt (unified runtime bridge).'
