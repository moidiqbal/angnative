import type { Metafile } from 'esbuild';

/**
 * Extract the package name from an esbuild metafile input path.
 * esbuild metafile paths look like:
 *   node_modules/.pnpm/@angular+core@22.2.5/node_modules/@angular/core/fesm2022/core.mjs
 *   or
 *   /abs/path/to/packages/testing/fixtures/out/app.js
 *
 * We map any @angular/* or @angnative/* path to its package name, and
 * everything else to "app" (the application code itself).
 */
function packageFromInput(inputPath: string): string {
  // pnpm layout: node_modules/.pnpm/@angular+core@22.2.5/node_modules/...
  // or node_modules/.pnpm/rxjs@7.8.2/node_modules/...
  // Package name uses '+' instead of '/', followed by '@version'.
  const match = inputPath.match(
    /node_modules[\/\\].pnpm[\/\\](@[^@]+|[^/@]+)@/
  );
  if (match) {
    return match[1]!.replace('+', '/');
  }
  // Flat layout: node_modules/@angular/core/... or node_modules/rxjs/...
  const flatMatch = inputPath.match(/node_modules[\/\\](@[^\/]+|[^\/]+)[\/\\]/);
  if (flatMatch) {
    return flatMatch[1]!.replace('+', '/');
  }
  return 'app';
}

/**
 * A single package's contribution to the bundle.
 */
export interface PackageSize {
  package: string;
  bytes: number;
  gzipBytes: number;
}

/**
 * Full size report: total + per-package breakdown.
 */
export interface SizeReport {
  totalBytes: number;
  totalGzipBytes: number;
  packages: PackageSize[];
  withinBudget: boolean;
  budgetBytes: number;
  forbiddenFound: string[];
}

/**
 * Build a size report from an esbuild metafile and gzip measurements.
 *
 * @param metafile - The esbuild metafile object
 * @param gzipBytes - Total gzipped size of the bundle (measured separately)
 * @param budgetBytes - The gzip budget to check against
 * @param forbiddenPackages - Package names that must not appear in the bundle
 */
export function buildSizeReport(
  metafile: Metafile,
  gzipBytes: number,
  budgetBytes: number,
  forbiddenPackages: string[] = []
): SizeReport {
  const packageBytes: Record<string, number> = {};

  for (const [inputPath, info] of Object.entries(metafile.inputs)) {
    const pkg = packageFromInput(inputPath);
    const current = packageBytes[pkg] ?? 0;
    packageBytes[pkg] = current + info.bytes;
  }

  const packages: PackageSize[] = Object.entries(packageBytes)
    .map(([pkgName, bytes]) => ({ package: pkgName, bytes, gzipBytes: 0 }))
    .sort((a, b) => b.bytes - a.bytes);

  // Distribute gzip bytes proportionally (exact per-package gzip is expensive;
  // the total gzip is the authoritative number).
  const totalRawBytes = packages.reduce((s, p) => s + p.bytes, 0);
  let remainingGzip = gzipBytes;
  for (let i = 0; i < packages.length; i++) {
    const pkg = packages[i]!;
    const share =
      i === packages.length - 1
        ? remainingGzip
        : Math.floor((pkg.bytes / totalRawBytes) * gzipBytes);
    pkg.gzipBytes = share;
    remainingGzip -= share;
  }

  // Check for forbidden packages in the metafile inputs
  const inputPaths = Object.keys(metafile.inputs);
  const forbiddenFound = forbiddenPackages.filter((fp) =>
    inputPaths.some((input) => input.includes(fp))
  );

  return {
    totalBytes: totalRawBytes,
    totalGzipBytes: gzipBytes,
    packages,
    withinBudget: gzipBytes <= budgetBytes && forbiddenFound.length === 0,
    budgetBytes,
    forbiddenFound,
  };
}
