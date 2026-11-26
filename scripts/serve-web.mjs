#!/usr/bin/env node
/**
 * serve-web.mjs — minimal dev server for the AngNative web adapter demo.
 *
 * Serves the AOT bundle produced by:
 *   pnpm compile:aot:web && pnpm bundle:aot:web
 * No framework dev-server and no JIT compiler are involved.
 */

import { createServer } from 'http';
import { existsSync, readFileSync, writeFileSync } from 'fs';
import { join } from 'path';

const root = process.cwd();
const outDir = join(root, 'dist', 'web-demo');
const bundlePath = join(outDir, 'app.js');
const htmlPath = join(outDir, 'index.html');
const port = Number(process.env.PORT || 4173);

if (!existsSync(bundlePath)) {
  console.error(`error: AOT web bundle not found: ${bundlePath}`);
  console.error('  Run: pnpm compile:aot:web && pnpm bundle:aot:web');
  process.exit(1);
}

const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8"/>
<title>AngNative Web Demo</title>
<style>body{font-family:system-ui,sans-serif;margin:24px;color:#1e293b}
na-view{display:flex}na-text{display:block}na-button{display:inline-flex;align-items:center;justify-content:center;
padding:8px 14px;border-radius:8px;background:#4f46e5;color:white;cursor:pointer;margin:4px}</style>
<script type="module" src="/app.js" fetchpriority="high"></script>
</head>
<body><demo-app></demo-app></body>
</html>`;
writeFileSync(htmlPath, html, 'utf-8');

createServer((req, res) => {
  const url = (req.url || '/').split('?')[0];
  const file = url === '/' || url === '/index.html'
    ? htmlPath
    : url === '/app.js'
      ? bundlePath
      : url === '/app.js.map'
        ? `${bundlePath}.map`
        : null;
  if (!file || !existsSync(file)) {
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
  console.log(`AngNative web demo (AOT): http://localhost:${port}`);
});
