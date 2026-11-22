# Sponsor AngNative — help us finish iOS (and keep the runtime independent)

AngNative is MIT-licensed and built in public. Everything except **iOS/macOS
verification** is implemented, tested and released in [`v0.2.0`](https://github.com/moidiqbal/angnative/releases/tag/v0.2.0):

- Rust host engine (124 tests + 6 QuickJS acceptance tests), Angular 22 renderer (208 tests), production AOT
- 10 native showcase APKs, Zero WebView, verified on a **OnePlus Open** (grid: 1.76% jank / 6 ms p50)
- Desktop (winit + wgpu) and web adapter running

The iOS shell is **code-complete** (Swift 6 package, C-ABI bridge, 8 UIKit view-factory
callbacks, tests) but needs a **Mac with Xcode** to build and run in Simulator. That
is the only blocker.

## How to sponsor

### 1. GitHub Sponsors (recommended — zero fees)
[![Sponsor](https://img.shields.io/badge/Sponsor-GitHub%20Sponsors-ea4aaa?style=for-the-badge&logo=githubsponsors&logoColor=white)](https://github.com/sponsors/moidiqbal)

GitHub matches the first $60/year for individual sponsors and there are no platform
fees for the first years. One-off or monthly both work.

### 2. Open Collective (transparent expenses)
[![Sponsor](https://img.shields.io/badge/Sponsor-Open%20Collective-3385FF?style=for-the-badge&logo=opencollective&logoColor=white)](https://opencollective.com/angnative)

Open Collective shows every expense publicly — ideal if you want to see exactly what
the money buys. Create the collective if it does not exist yet; the FUNDING.yml link
is already wired.

### 3. Buy Me a Coffee / Ko-fi (one-off)
For a quick one-off coffee, use any of the links above's "one-time" option.

## What the money buys (transparent)

| Goal | Approx. cost | Status |
|---|---:|---|
| macOS CI runner (GitHub Actions macOS minutes) or a used Mac mini (M1/M2) for Xcode | $30–60/mo or ~$400–700 one-off | ⬜ needed now |
| Apple Developer Program (device signing / TestFlight) | $99/yr | ⬜ needed for device release |
| Physical iPhone for final on-device verification | variable (donation) | 🟢 nice-to-have |

All expenses will be recorded in `docs/SPONSORS.md` and reflected in release notes.

## Recognition

Every sponsor gets listed here (name/handle + link, unless anonymous) and in the
release notes. Companies sponsoring the Mac or CI runner get a "Sponsor" section in
the README with their logo.

## If you can lend Mac time instead of money

Even a 30-minute session is enough:
```bash
./scripts/build-ios-native.sh simulator
cd platforms/ios && swift test -Xlinker -L../../platforms/ios/Sources/AngNativeHost
```
Then share the output/screenshots in an issue titled **"iOS Mac verification"**.
That closes the last gap in v0.2.0.

## Current sponsors

_none yet — be the first to help us ship iOS._
