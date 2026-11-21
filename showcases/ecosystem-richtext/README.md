# AngNative — Native Rich Text Ecosystem Showcase

A **Phase 2 ecosystem showcase** demonstrating a genuine native rich-text editor.

## Honest framing

**Tiptap and CKEditor are DOM/JavaScript libraries.** They require a browser
document, `contentEditable`, and CSS and therefore cannot run unchanged inside a
native Android shell. This project deliberately has **no WebView and no JS
engine**. It is a **native reimplementation / equivalent**, built with real
Android widgets and spans — not unchanged Tiptap/CKEditor compatibility.

## What is implemented

- `EditText` + `SpannableString` / `SpannableStringBuilder` backed editor.
- Real span objects: `StyleSpan` (bold/italic), `UnderlineSpan`,
  `ForegroundColorSpan`, `AbsoluteSizeSpan`, `BulletSpan`, `URLSpan`.
- Toolbar: Bold, Italic, Underline, Bullet list, Text color (cycles Red → Blue →
  Green), A+ / A- font size, Undo (stack of text+span snapshots), Clear.
- Live **format summary** listing the spans that cover the current
  cursor/selection.
- The Rust JNI bridge (`JniBridge.kt` / `jni_bridge.cpp` / prebuilt
  `libangnative_host_ffi.a`) is kept intact and is exercised at startup:
  `AngNative host created` in logcat plus a dispatched batch and its response.

## Coordinates

- Package id: `com.angnative.demo.ecosystem.richtext`
- Kotlin/JNI JVM package: `com.angnative.app` (unchanged, so JNI symbols such as
  `Java_com_angnative_app_JniBridge_nativeHostCreate` keep resolving)
- Launch component: `com.angnative.demo.ecosystem.richtext/com.angnative.app.MainActivity`

## Build

```powershell
$env:ANDROID_HOME = "C:\Android\Sdk"
$env:JAVA_HOME   = "C:\Program Files\Java\jdk-25.0.5.1"
cd showcases\ecosystem-richtext
.\gradlew.bat assembleDebug --no-daemon
```

`local.properties` contains `sdk.dir=C:/Android/Sdk`.

## Verification / artifacts

- APK: `dist/showcases/ecosystem-richtext/app-debug.apk`
- Report: `dist/showcases/ecosystem-richtext/compatibility-report.md`
- Sources: `dist/showcases/ecosystem-richtext/source.zip`
- Screenshot: `docs/assets/showcase-ecosystem-richtext.png`

## Limitations

- Native reimplementation, not drop-in DOM-library compatibility.
- No HTML import/export: document state is a `Spannable`, not HTML/ProseMirror JSON.
- `URLSpan` is applied in the seeded sample; there is no "insert link" dialog.
- Undo is a bounded (40-entry) snapshot stack, not a full transaction history.
- No rich-text collaboration, schema validation, or plugin API.
