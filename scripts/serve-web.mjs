#!/usr/bin/env node
/**
 * serve-web.mjs — minimal dev server for the AngNative web adapter demo.
 * Bundles apps/demo-app/src/main.web.ts with esbuild and serves it over HTTP.
 * No framework dev-server dependency is required.
 */

import { createServer } from 'http';
import { mkdirSync, writeFileSync, readFileSync } from 'fs';
import { join } from 'path';
import { build } from 'esbuild';

const root = process.cwd();
const outDir = join(root, 'dist', 'web-demo');
const bundlePath = join(outDir, 'app.js');
const port = Number(process.env.PORT || 4173);

await mkdirSync(outDir, { recursive: true });

await build({
  entryPoints: [join(root, 'apps', 'demo-app', 'src', 'main.web.ts')],
  outfile: bundlePath,
  bundle: true,
  format: 'esm',
  platform: 'browser',
  target: 'es2022',
  sourcemap: true,
  logLevel: 'info',
});

const html = `<!doctype html>
<html lang="en">
<head><meta charset="utf-8"/><title>AngNative Web Demo</title>
<style>body{font-family:system-ui,sans-serif;margin:24px;color:#1e293b}
na-view{display:flex}na-text{display:block}na-button{display:inline-flex;align-items:center;justify-content:center;
padding:8px 14px;border-radius:8px;background:#4f46e5;color:white;cursor:pointer;margin:4px}</style>
</head>
<body><demo-app></demo-app><script type="module" src="/app.js"></script></body>
</html>`;
writeFileSync(join(outDir, 'index.html'), html, 'utf-8');

createServer((req, res) => {
  const url = (req.url || '/').split('?')[0];
  const file = url === '/' || url === '/index.html'
    ? join(outDir, 'index.html')
    : url === '/app.js'
      ? bundlePath
      : url === '/app.js.map'
        ? `${bundlePath}.map`
        : null;
  if (!file) {
    res.writeHead(404).end('not found');
    return;
  }
  try {
    const body = readFileSync(file);
    res.writeHead(200, {
      'content-type': file.endsWith('.html') ? 'text/html' : 'text/javascript',
    });
    res.end(body);
  } catch {
    res.writeHead(404).end('not found');
  }
}).listen(port, () => {
  console.log(`AngNative web demo: http://localhost:${port}`);
});
