/**
 * conformance.ts — fixture harness helpers.
 */

import { readFileSync } from 'fs';
import { join } from 'path';
import { fileURLToPath } from 'url';
import { FakeBackend } from './fake-backend';

const __dirname = fileURLToPath(new URL('.', import.meta.url)).replace(/%20/g, ' ');
const FIXTURES_DIR = join(__dirname, '..', 'fixtures');

export interface Fixture {
  name: string;
  description: string;
  commands: any[];
  expect?: any;
}

export function loadManifest(): string[] {
  const raw = readFileSync(join(FIXTURES_DIR, 'manifest.json'), 'utf8');
  return JSON.parse(raw) as string[];
}

export function loadFixture(name: string): Fixture {
  const raw = readFileSync(join(FIXTURES_DIR, `${name}.json`), 'utf8');
  return JSON.parse(raw) as Fixture;
}

export function runFixture(name: string) {
  const fixture = loadFixture(name);
  const backend = new FakeBackend();
  backend.runBatch(fixture.commands);
  return { fixture, result: backend.snapshot() };
}
