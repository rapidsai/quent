// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { readdir, readFile, stat } from 'node:fs/promises';
import path from 'node:path';

export function caseKey(value) {
  return JSON.stringify([value.language ?? 'rust', value.implementation, value.exporter, value.event_shape, value.threads]);
}

export function validateReport(report) {
  if (!report || typeof report !== 'object' || !report.system || !Array.isArray(report.cases) || !report.cases.length) {
    throw new Error('report needs system and cases');
  }
  if (!Number.isSafeInteger(report.system.captured_at_unix_seconds) || report.system.captured_at_unix_seconds < 0) {
    throw new Error('report needs a valid capture time');
  }
  if (typeof report.system.os !== 'string' || typeof report.system.architecture !== 'string' ||
      !(report.system.cpu_model === null || typeof report.system.cpu_model === 'string') ||
      !(report.system.available_cpu_count === null ||
        (Number.isSafeInteger(report.system.available_cpu_count) && report.system.available_cpu_count > 0)) ||
      !(report.system.build_profile === null || typeof report.system.build_profile === 'string')) {
    throw new Error('report needs valid system metadata');
  }
  const keys = new Set();
  for (const item of report.cases) {
    if (!item || typeof item !== 'object' || typeof item.implementation !== 'string' || !item.implementation ||
        !(item.language === undefined || ['rust', 'cpp', 'python'].includes(item.language)) ||
        !Number.isSafeInteger(item.threads) || item.threads < 1 ||
        !Number.isSafeInteger(item.num_batches) || item.num_batches < 1 ||
        !Number.isSafeInteger(item.batch_size) || item.batch_size < 1 ||
        !Number.isFinite(item.average_ns_per_iteration) || item.average_ns_per_iteration < 0 ||
        !Array.isArray(item.thread_batch_elapsed_ns) ||
        item.thread_batch_elapsed_ns.length !== item.threads ||
        item.thread_batch_elapsed_ns.some((thread) => !Array.isArray(thread) ||
          thread.length !== item.num_batches ||
          thread.some((duration) => !Number.isSafeInteger(duration) || duration < 0))) {
      throw new Error('invalid case or batch timing');
    }
    if (!(item.exporter === null || typeof item.exporter === 'string') ||
        !(item.event_shape === null || typeof item.event_shape === 'string') ||
        (item.event_shape === null && item.exporter !== null)) {
      throw new Error('cases need a valid event shape and optional exporter');
    }
    const key = caseKey(item);
    if (keys.has(key)) throw new Error('duplicate case');
    keys.add(key);
  }
  return report;
}

export async function readReports(directory, warn = () => {}, cache = new Map()) {
  let entries;
  try {
    entries = await readdir(directory, { withFileTypes: true });
  } catch (error) {
    if (error.code === 'ENOENT') return [];
    throw error;
  }
  const names = new Set(entries.filter((entry) => entry.isFile() && entry.name.endsWith('.json')).map((entry) => entry.name));
  for (const name of cache.keys()) {
    if (!names.has(name)) cache.delete(name);
  }
  const reports = await Promise.all(entries.filter((entry) => entry.isFile() && entry.name.endsWith('.json'))
    .map(async (entry) => {
      const file = path.join(directory, entry.name);
      try {
        const metadata = await stat(file);
        const version = `${metadata.size}-${metadata.mtimeMs}`;
        if (cache.get(entry.name)?.version === version) return cache.get(entry.name);
        const content = await readFile(file, 'utf8');
        const result = { name: entry.name, file, version, report: validateReport(JSON.parse(content)) };
        cache.set(entry.name, result);
        return result;
      } catch (error) {
        warn(`${entry.name}: ${error.message}`);
        return null;
      }
    }));
  return reports.filter(Boolean).sort((a, b) =>
    b.report.system.captured_at_unix_seconds - a.report.system.captured_at_unix_seconds ||
    b.name.localeCompare(a.name));
}

export function selectLatestReport(reports) {
  return reports[0] ?? null;
}
