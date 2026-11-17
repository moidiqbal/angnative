/**
 * create.ts — `angnative create <app-name>` scaffolding.
 *
 * Scaffolds a new AngNative application directory with:
 *   package.json   — workspace member, @angnative deps
 *   tsconfig.json  — extends the monorepo base
 *   vitest.config.ts — test runner config
 *   src/main.ts    — bootstrap entry via @angnative/platform
 *   src/app.component.ts — root component using na-* elements
 *
 * No per-platform forks — the same scaffold targets Android, iOS,
 * Desktop, and Web; only the platform shell differs.
 */

import { mkdirSync, writeFileSync, existsSync } from 'fs';
import { join, resolve, dirname } from 'path';

/** Files to scaffold for a new app. */
interface ScaffoldFile {
  readonly path: string;
  readonly content: string;
}

/** package.json for the new app. */
function appPackageJson(appName: string): string {
  return JSON.stringify(
    {
      name: appName,
      version: '0.1.0',
      private: true,
      type: 'module',
      license: 'MIT',
      description: `AngNative app: ${appName}`,
      scripts: {
        start: 'echo "Run on a platform shell: android | ios | desktop | web"',
        test: 'vitest run',
      },
      dependencies: {
        '@angular/core': '22.2.5',
        '@angular/common': '22.2.5',
        '@angnative/core': '0.1.0',
        '@angnative/common': '0.1.0',
        '@angnative/platform': '0.1.0',
        '@angnative/contract': '0.1.0',
      },
      devDependencies: {
        vitest: '3.2.4',
      },
    },
    null,
    2,
  );
}

/** tsconfig.json for the new app. */
function appTsConfig(): string {
  return JSON.stringify(
    {
      extends: '../../tsconfig.base.json',
      compilerOptions: {
        outDir: './dist',
        rootDir: './src',
      },
      include: ['src/**/*.ts'],
    },
    null,
    2,
  );
}

/** vitest.config.ts for the new app. */
function appVitestConfig(): string {
  return `import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['src/**/*.spec.ts'],
  },
});
`;
}

/** src/main.ts — bootstrap entry. */
function appMainTs(): string {
  return `import { bootstrapNativeAngularApp } from '@angnative/platform';
import { AppComponent } from './app.component';

bootstrapNativeAngularApp(AppComponent, {
  providers: [],
  zoneless: true,
}).then((ref) => {
  console.log('[angnative] app bootstrapped on native renderer');
}).catch((err) => {
  console.error('[angnative] bootstrap failed:', err);
});
`;
}

/** src/app.component.ts — root component using na-* elements. */
function appComponentTs(kebab: string): string {
  return `import { Component } from '@angular/core';
import {
  NaViewComponent as NaView,
  NaTextComponent as NaText,
} from '@angnative/common';

@Component({
  selector: '${kebab}',
  standalone: true,
  imports: [NaView, NaText],
  template: \`
    <na-view [style.flexDirection]="'column'" [style.padding]="'24px'" [style.gap]="'16px'">
      <na-text [style.fontSize]="'20px'" [style.fontWeight]="'bold'">Welcome to AngNative</na-text>
      <na-text [style.color]="'#666'">Angular running natively — no WebView.</na-text>
    </na-view>
  \`,
})
export class AppComponent {}
`;
}

function kebabCase(name: string): string {
  return name
    .replace(/[^a-zA-Z0-9]+/g, '-')
    .replace(/([a-z])([A-Z])/g, '$1-$2')
    .toLowerCase()
    .replace(/^-+|-+$/g, '');
}

/** Build the list of files to scaffold. */
function scaffoldFiles(appName: string, targetDir: string): ScaffoldFile[] {
  const kebab = kebabCase(appName);
  const componentTs = appComponentTs(kebab);

  return [
    { path: join(targetDir, 'package.json'), content: appPackageJson(kebab) },
    { path: join(targetDir, 'tsconfig.json'), content: appTsConfig() },
    { path: join(targetDir, 'vitest.config.ts'), content: appVitestConfig() },
    { path: join(targetDir, 'src', 'main.ts'), content: appMainTs() },
    { path: join(targetDir, 'src', 'app.component.ts'), content: componentTs },
  ];
}

/**
 * Scaffold a new AngNative app at `<cwd>/<appName>`.
 * Returns the absolute path to the created directory.
 */
export function createApp(appName: string): string {
  if (!appName || !appName.trim()) {
    throw new Error('app name is required: angnative create <app-name>');
  }

  const kebab = kebabCase(appName);
  const targetDir = resolve(process.cwd(), kebab);

  if (existsSync(targetDir)) {
    throw new Error(`directory already exists: ${targetDir}`);
  }

  const files = scaffoldFiles(kebab, targetDir);

  for (const file of files) {
    mkdirSync(dirname(file.path), { recursive: true });
    writeFileSync(file.path, file.content, 'utf8');
  }

  return targetDir;
}

/** CLI entry point: `angnative create <app-name>`. */
export function runCreate(argv: string[]): number {
  const [appName] = argv;

  if (!appName) {
    console.error('error: app name is required');
    console.log('  Usage: angnative create <app-name>');
    return 1;
  }

  try {
    const dir = createApp(appName);
    console.log(`[angnative create] scaffolded ${appName} at ${dir}`);
    console.log(`  Next: cd ${appName} && pnpm install`);
    return 0;
  } catch (err) {
    console.error(`error: create failed: ${err instanceof Error ? err.message : String(err)}`);
    return 1;
  }
}

export { kebabCase };
