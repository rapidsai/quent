// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { copyFile, mkdir, mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { build, createServer } from 'vite';
import { assetReferences, inlineSinglePage } from './inline.mjs';
import { readReports, selectLatestReport } from './reports.mjs';

const root = fileURLToPath(new URL('..', import.meta.url));
const repositoryRoot = path.resolve(root, '..', '..', '..');
const command = process.argv[2];
const options = process.argv.slice(3);
const option = (name, fallback) => {
  const position = options.indexOf(name);
  if (position === -1) return fallback;
  if (!options[position + 1]) throw new Error(`${name} needs a path`);
  return path.resolve(repositoryRoot, options[position + 1]);
};
const results = option('--results', path.resolve(repositoryRoot, 'benchmarks', 'results'));

if (command === 'serve') {
  process.env.QUENT_BENCH_RESULTS = results;
  const server = await createServer({ root, configFile: path.join(root, 'vite.config.ts') });
  await server.listen();
  server.printUrls();
} else if (command === 'build') {
  const output = option('--output', path.resolve(repositoryRoot, 'benchmarks', 'results', 'site'));
  const current = selectLatestReport(await readReports(results, console.warn));
  if (!current) throw new Error(`No valid reports in ${results}`);
  if (output === results || output === root || results.startsWith(`${output}${path.sep}`) || root.startsWith(`${output}${path.sep}`)) {
    throw new Error('Output must not contain the results directory or app source');
  }
  const existing = await readdir(output).catch((error) => {
    if (error.code === 'ENOENT') return [];
    throw error;
  });
  if (existing.length && (!existing.includes('.quent-bench-site') ||
      (await readFile(path.join(output, '.quent-bench-site'), 'utf8')) !== 'quent-bench-plot\n')) {
    throw new Error(`Refusing to replace an output directory not built by quent-bench-plot: ${output}`);
  }
  await build({ root, configFile: path.join(root, 'vite.config.ts'), build: { outDir: output, emptyOutDir: true } });
  await mkdir(output, { recursive: true });
  await writeFile(path.join(output, '.quent-bench-site'), 'quent-bench-plot\n');
  await copyFile(current.file, path.join(output, 'current.json'));
  await writeFile(path.join(output, 'selection.json'), JSON.stringify({
    current: { url: './current.json', version: current.version },
  }));
  console.log(`Built ${output}`);
} else if (command === 'build-single') {
  const output = option('--output', path.resolve(results, 'benchmark-plot.html'));
  const current = selectLatestReport(await readReports(results, console.warn));
  if (!current) throw new Error(`No valid reports in ${results}`);
  const marker = '<!-- quent-bench-single-page -->';
  const existing = await readFile(output, 'utf8').catch((error) => {
    if (error.code === 'ENOENT') return null;
    throw error;
  });
  if (existing !== null && !existing.includes(marker)) {
    throw new Error(`Refusing to replace a file not built by quent-bench-plot: ${output}`);
  }
  const temporary = await mkdtemp(path.join(os.tmpdir(), 'quent-bench-plot-'));
  try {
    await build({ root, configFile: path.join(root, 'vite.config.ts'),
      build: { outDir: temporary, emptyOutDir: true, assetsInlineLimit: Number.MAX_SAFE_INTEGER } });
    let html = await readFile(path.join(temporary, 'index.html'), 'utf8');
    const assets = await readdir(path.join(temporary, 'assets'));
    if (assets.length !== 2) {
      throw new Error('Unexpected Vite output for single-page build');
    }
    const { script, stylesheet } = assetReferences(html);
    const javascript = await readFile(path.join(temporary, 'assets', script[1]), 'utf8');
    const css = await readFile(path.join(temporary, 'assets', stylesheet[1]), 'utf8');
    html = inlineSinglePage(html, javascript, css, await readFile(current.file, 'utf8'));
    await mkdir(path.dirname(output), { recursive: true });
    await writeFile(output, html);
    console.log(`Built ${output}`);
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
} else {
  throw new Error('Usage: site.mjs serve|build|build-single [--results DIR] [--output PATH]');
}
