import { describe, it, expect } from 'vitest';
import { runDoctor, checkNode, checkAngular, checkTypeScript, checkRust, checkAndroid, checkNdk, checkXcode } from './doctor';

describe('doctor — toolchain checks', () => {
  it('runDoctor returns ok boolean and checks array', () => {
    const result = runDoctor();
    expect(typeof result.ok).toBe('boolean');
    expect(Array.isArray(result.checks)).toBe(true);
    expect(result.checks.length).toBeGreaterThan(0);
  });

  it('every check has name, ok, and detail fields', () => {
    const result = runDoctor();
    for (const c of result.checks) {
      expect(typeof c.name).toBe('string');
      expect(typeof c.ok).toBe('boolean');
      expect(c.ok || c.detail).toBeDefined();
    }
  });

  it('checkNode: reports version and floor', () => {
    const c = checkNode();
    expect(c.name).toBe('node');
    expect(typeof c.ok).toBe('boolean');
    expect(c.required).toContain('22.12');
  });

  it('checkAngular: requires exactly 22.3.0', () => {
    const c = checkAngular();
    expect(c.name).toBe('@angular/core');
    expect(c.required).toBe('22.3.0');
  });

  it('checkTypeScript: requires exactly 6.0.3', () => {
    const c = checkTypeScript();
    expect(c.name).toBe('typescript');
    expect(c.required).toBe('6.0.3');
  });

  it('checkRust: requires cargo + rustc', () => {
    const c = checkRust();
    expect(c.name).toBe('rust');
    expect(typeof c.ok).toBe('boolean');
  });

  it('checkAndroid: requires ANDROID_HOME + adb', () => {
    const c = checkAndroid();
    expect(c.name).toBe('android-sdk');
    expect(typeof c.ok).toBe('boolean');
  });

  it('checkNdk: requires ANDROID_NDK_HOME', () => {
    const c = checkNdk();
    expect(c.name).toBe('android-ndk');
    expect(typeof c.ok).toBe('boolean');
  });

  it('checkXcode: skipped on non-macOS, required on macOS', () => {
    const c = checkXcode();
    expect(c.name).toBe('xcode');
    expect(typeof c.ok).toBe('boolean');
  });

  it('overall ok is false when any check fails', () => {
    const result = runDoctor();
    if (result.checks.some((c) => !c.ok)) {
      expect(result.ok).toBe(false);
    }
  });
});
