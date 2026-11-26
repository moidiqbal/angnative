/**
 * Angular Linker plugins for esbuild and Vite/Vitest.
 *
 * Angular packages under node_modules ship partial-compilation declarations
 * (`ɵɵngDeclareFactory`, etc.). A build must run the Angular Linker over those
 * files or the runtime falls back to JIT. These plugins link them without
 * importing @angular/compiler, keeping the AngNative web demo AOT-only.
 */
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const require = createRequire(import.meta.url);
const pnpmStore = join(root, 'node_modules', '.pnpm', 'node_modules');

const { transformSync } = require(require.resolve('@babel/core', {
  paths: [pnpmStore],
}));
const linkerModule = require(require.resolve('@angular/compiler-cli/linker/babel'));
const angularLinkerPlugin = linkerModule.default ?? linkerModule;

const PARTIAL_MARKER = 'ɵɵngDeclare';

function linkIfPartial(code, filename) {
  if (typeof code !== 'string' || !code.includes(PARTIAL_MARKER)) return null;
  const result = transformSync(code, {
    filename,
    babelrc: false,
    configFile: false,
    sourceType: 'module',
    compact: false,
    plugins: [[angularLinkerPlugin, { linkerJitMode: false }]],
  });
  if (!result?.code) {
    throw new Error(`Angular Linker produced no output for ${filename}`);
  }
  return result.code;
}

/** esbuild plugin: link Angular partial declarations while loading modules. */
export function angularLinkerEsbuildPlugin() {
  return {
    name: 'angnative-angular-linker',
    setup(build) {
      build.onLoad({ filter: /\.[cm]?js$/ }, async (args) => {
        const code = await readFile(args.path, 'utf8');
        const linked = linkIfPartial(code, args.path);
        return linked === null ? null : { contents: linked, loader: 'js' };
      });
    },
  };
}

/** Vite/Vitest plugin: link Angular partial declarations in the module graph. */
export function angularLinkerVitePlugin() {
  return {
    name: 'angnative-angular-linker',
    enforce: 'pre',
    transform(code, id) {
      // Angular partial declarations live in compiled JS, never in .ts specs.
      // A spec may mention the marker as a literal string; transforming it
      // with Babel's JS parser would fail on TypeScript syntax.
      const filename = id.split('?')[0];
      if (!/\.[cm]?js$/.test(filename)) return null;
      const linked = linkIfPartial(code, id);
      return linked === null ? null : { code: linked, map: null };
    },
  };
}

export const angularLinkerInlineDeps = [
  /@angular\/(common|compiler|core|platform-browser|platform-browser-dynamic)/,
];
