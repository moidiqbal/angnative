import { build, type BuildOptions, type Metafile } from 'esbuild';
import { readFileSync, writeFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Angular production defines for dead-code elimination.
 * Setting ngDevMode: false tells @angular/core to drop development-only code,
 * including the JIT compiler paths, debug assertions, and instrumentation.
 * This is what reduces the payload from ~120 KB gzip to ~90 KB gzip.
 */
const ANGULAR_PRODUCTION_DEFINES: Record<string, string> = {
  'ngDevMode': 'false',
  'ngI18nClosureMode': 'false',
  'ngJitMode': 'false',
  'ngRuntimeStrictMode': 'true',
  'ngDevModeAndNotTest': 'false',
  'ngBrowserAnimation': 'false',
};

/**
 * Bundle an AOT-compiled Angular app for production.
 *
 * @param entry - Absolute path to the AOT-compiled entry (e.g. app.js)
 * @param outfile - Absolute path for the output bundle
 * @param options - Additional esbuild options (merged after defaults)
 * @returns The esbuild build result, including the metafile
 */
export async function bundleApp(
  entry: string,
  outfile: string,
  options: Partial<BuildOptions> = {}
): Promise<{ metafile: Metafile; outputPath: string }> {
  const outDir = dirname(outfile);
  const metafilePath = join(outDir, 'bundle.meta.json');

  const buildOptions: BuildOptions = {
    entryPoints: [entry],
    outfile,
    bundle: true,
    format: 'esm',
    platform: 'browser',
    target: 'es2022',
    sourcemap: false,
    minify: true,
    treeShaking: true,
    metafile: true,
    write: true,
    define: ANGULAR_PRODUCTION_DEFINES,
    packages: 'external',
    ...options,
  };

  const result = await build(buildOptions);

  if (!result.metafile) {
    throw new Error('esbuild did not produce a metafile. Cannot verify size budget without it.');
  }

  // Persist the metafile so the size gate can read it later
  writeFileSync(metafilePath, JSON.stringify(result.metafile, null, 2));

  return { metafile: result.metafile, outputPath: outfile };
}

/**
 * Get the gzipped size of a file using Node's zlib.
 */
export async function gzipSize(filePath: string): Promise<number> {
  const { createGzip } = await import('zlib');
  const input = readFileSync(filePath);
  const gzipped = await new Promise<Buffer>((resolve, reject) => {
    const gzip = createGzip();
    const chunks: Buffer[] = [];
    gzip.on('data', (chunk: Buffer) => chunks.push(chunk));
    gzip.on('end', () => resolve(Buffer.concat(chunks)));
    gzip.on('error', reject);
    gzip.end(input);
  });
  return gzipped.length;
}

/**
 * Read a persisted metafile from disk.
 */
export function readMetafile(metafilePath: string): Metafile {
  if (!existsSync(metafilePath)) {
    throw new Error(`Metafile not found at ${metafilePath}`);
  }
  return JSON.parse(readFileSync(metafilePath, 'utf-8')) as Metafile;
}
