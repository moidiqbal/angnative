import { describe, it, expect } from 'vitest';
import { readFileSync, existsSync, readdirSync } from 'fs';
import { join, dirname, relative } from 'path';
import { fileURLToPath } from 'url';

const REPO_ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');
const REGISTER = join(REPO_ROOT, 'docs', 'UNSUPPORTED-FEATURES.md');

/** Every workspace manifest, found from the repo root (not the test cwd). */
function packageManifests(): Array<{ file: string; json: any }> {
  const files: string[] = [join(REPO_ROOT, 'package.json')];
  for (const group of ['apps', 'packages']) {
    const base = join(REPO_ROOT, group);
    if (!existsSync(base)) continue;
    for (const name of readdirSync(base)) {
      const file = join(base, name, 'package.json');
      if (existsSync(file)) files.push(file);
    }
  }
  return files.map((file) => ({ file, json: JSON.parse(readFileSync(file, 'utf-8')) }));
}

function declaredNames(manifest: any): string[] {
  const names: string[] = [];
  for (const section of ['dependencies', 'devDependencies', 'peerDependencies']) {
    const deps = manifest[section];
    if (deps && typeof deps === 'object') names.push(...Object.keys(deps));
  }
  return names;
}

describe('scope guards — documented deferrals, not runtime claims', () => {
  it('keeps SSR/hydration formally unsupported and undeclared', () => {
    const register = readFileSync(REGISTER, 'utf-8');
    expect(register).toContain('| SSR / hydration | **Unsupported (out of scope)** |');
    for (const { file, json } of packageManifests()) {
      expect(
        declaredNames(json).filter((name) => name === '@angular/platform-server'),
        `unexpected SSR dependency in ${relative(REPO_ROOT, file)}`,
      ).toEqual([]);
    }
    expect(existsSync(join(REPO_ROOT, 'docs', 'evidence', 'SSR-HYDRATION-UNSUPPORTED.md'))).toBe(true);
  });

  it('keeps DOM-heavy libraries formally unsupported and undeclared', () => {
    const register = readFileSync(REGISTER, 'utf-8');
    expect(register).toContain('| DOM-heavy libraries unchanged');
    expect(register).toContain('**Unsupported on native path**');
    // RxJS remains explicitly a DOM-free spot-check, not general compatibility.
    expect(register).toContain('It is a **spot-check, not general compatibility**');

    const forbidden = [
      '@angular/material',
      'primeng',
      '@ng-zorro/ng-zorro-antd',
      '@taiga-ui/core',
      '@spartan-ng/ui-core',
      'ag-grid-community',
      'ag-grid-angular',
      'echarts',
      'ngx-echarts',
      'tiptap',
      '@tiptap/core',
      'ckeditor',
      'fullcalendar',
      '@fullcalendar/core',
      'gsap',
    ];
    for (const { file, json } of packageManifests()) {
      const found = declaredNames(json).filter((name) =>
        forbidden.some((item) => name === item || name.startsWith(`${item}/`)),
      );
      expect(found, `unexpected DOM-heavy dependency in ${relative(REPO_ROOT, file)}`).toEqual([]);
    }
    expect(existsSync(join(REPO_ROOT, 'docs', 'evidence', 'DOM-PARITY-UNSUPPORTED.md'))).toBe(true);
  });

  it('keeps the StubEngine evidence distinct from real engine integration', () => {
    const evidence = readFileSync(
      join(REPO_ROOT, 'docs', 'evidence', 'ENGINE-STUB-CONFORMANCE.md'),
      'utf-8',
    );
    expect(evidence).toContain('non-executing');
    expect(evidence).toContain('not integrated');
    expect(evidence).toContain('--features engine-stub');
  });
});
