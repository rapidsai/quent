// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { spawn } from 'node:child_process';
import { readdir, readFile, rename, rm, stat, mkdir } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const uiRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const recordingsDir = path.join(uiRoot, 'test-results', 'demo-gif');

function outputPath() {
  const args = process.argv.slice(2);
  const outputIndex = args.indexOf('--output');
  if (outputIndex === -1) {
    return path.join(uiRoot, 'docs', 'screenshots', 'demo.gif');
  }
  const value = args[outputIndex + 1];
  if (!value) {
    throw new Error('--output requires a path');
  }
  return path.resolve(process.cwd(), value);
}

async function run(command: string, args: string[], env: NodeJS.ProcessEnv = process.env) {
  await new Promise<void>((resolve, reject) => {
    const child = spawn(command, args, { cwd: uiRoot, env, stdio: 'inherit' });
    child.on('error', reject);
    child.on('exit', (code, signal) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`${command} failed with ${signal ?? `exit code ${code}`}`));
      }
    });
  });
}

async function findVideos(directory: string): Promise<string[]> {
  const entries = await readdir(directory, { withFileTypes: true });
  const videos: string[] = [];
  for (const entry of entries) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      videos.push(...(await findVideos(entryPath)));
    } else if (entry.name.endsWith('.webm')) {
      videos.push(entryPath);
    }
  }
  return videos;
}

async function main() {
  const output = outputPath();
  await rm(recordingsDir, { force: true, recursive: true });
  await run('pnpm', ['exec', 'playwright', 'test', '--config', 'playwright.demo.config.ts'], {
    ...process.env,
    PLAYWRIGHT_DEMO_RECORD: '1',
  });

  const videos = await findVideos(recordingsDir);
  if (videos.length !== 1) {
    throw new Error(`Expected one Playwright video, found ${videos.length}`);
  }

  await mkdir(path.dirname(output), { recursive: true });
  const temporaryOutput = path.join(path.dirname(output), `.${path.basename(output)}.tmp.gif`);
  await rm(temporaryOutput, { force: true });
  await run('ffmpeg', [
    '-y',
    '-ss',
    '1',
    '-i',
    videos[0]!,
    '-filter_complex',
    [
      '[0:v]fps=6,scale=1200:-2:flags=lanczos,split[source][palette_source]',
      '[palette_source]palettegen=max_colors=80:stats_mode=diff[palette]',
      '[source][palette]paletteuse=dither=bayer:bayer_scale=3:diff_mode=rectangle',
    ].join(';'),
    '-loop',
    '0',
    temporaryOutput,
  ]);

  const header = (await readFile(temporaryOutput)).subarray(0, 6);
  const size = (await stat(temporaryOutput)).size;
  if (!header.toString('ascii').startsWith('GIF8') || size < 100_000) {
    throw new Error(`Generated GIF is invalid or unexpectedly small (${size} bytes)`);
  }

  await rename(temporaryOutput, output);
  process.stdout.write(`Wrote ${(size / 1_000_000).toFixed(1)} MB to ${output}\n`);
}

await main();
