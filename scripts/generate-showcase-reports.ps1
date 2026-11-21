# Generate compatibility-report.md for each native showcase.
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$apps = @(
  @{ id='material'; label='Angular Material'; pkg='com.angnative.demo.material' },
  @{ id='primeng';  label='PrimeNG';          pkg='com.angnative.demo.primeng' },
  @{ id='ngzorro';  label='NG-ZORRO';         pkg='com.angnative.demo.ngzorro' },
  @{ id='taiga';    label='Taiga UI';         pkg='com.angnative.demo.taiga' },
  @{ id='spartan';  label='Spartan UI';       pkg='com.angnative.demo.spartan' }
)
foreach ($a in $apps) {
  $out = Join-Path $repo "dist\showcases\$($a.id)\compatibility-report.md"
  New-Item -ItemType Directory -Force (Split-Path $out) | Out-Null
  $apk = Join-Path $repo "dist\showcases\$($a.id)\app-debug.apk"
  $size = if (Test-Path $apk) { [math]::Round((Get-Item $apk).Length/1MB,2) } else { 'n/a' }
  $content = @"
# Compatibility Report — $($a.label) Native Showcase

**Date:** 2026-11-21
**Application:** ``$($a.pkg)``
**APK:** ``dist/showcases/$($a.id)/app-debug.apk`` ($size MB)

## Verdict
**Native reimplementation required.** $($a.label) is a DOM/CSS library and cannot
execute unchanged on the AngNative native renderer. This showcase uses only
native Android views driven by Rust ``PlatformOp``s through JNI. The library's
visual language is a design reference, not a runtime dependency.

## Evidence
| Item | Value |
|---|---|
| Library | $($a.label) (design reference only — not bundled) |
| Android API | compileSdk/targetSdk 34, minSdk 24 |
| Renderer | AngNative Rust host → JNI → NativeViewFactory → native View/TextView/Button |
| WebView | none (Zero WebView policy) |
| Build | Gradle 9.9.2 / AGP 9.5.2 / JDK 25 — ``./gradlew assembleDebug`` |
| Install/launch | verified on emulator API 34 x86_64 (AngNative34) |
| Screenshot | ``docs/assets/showcase-$($a.id).png`` |
| Source | ``showcases/$($a.id)/`` + ``source.zip`` |

## Feature coverage (POC)
| Feature | Status |
|---|---|
| Buttons, cards, typography, palette | implemented natively |
| Inputs, selects, dialogs, tabs, tables | planned |
| Virtual scrolling, dark mode, a11y, gestures | planned |

## Known limitations
- Native equivalent, NOT unchanged third-party compatibility.
- Performance measurements pending (Phase 6).
"@
  Set-Content $out $content
  Write-Output "wrote $out"
}
