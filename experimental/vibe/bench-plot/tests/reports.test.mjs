// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { readReports, selectLatestReport, validateReport } from '../scripts/reports.mjs';
import { availableLanguages, batchAverages, boxSummary, caseAxisLabel, caseLabel, discardedCallCount, frameworkColor, frameworkColors, isNoopCase, payloadGroups, sortCasesByAverage } from '../src/report.ts';
import { jitterOffset } from '../src/jitter.ts';

function report(time, settings = {}) {
  return {
    system: { captured_at_unix_seconds: time, os: 'linux', architecture: 'x86_64',
      cpu_model: 'test CPU', available_cpu_count: 4, build_profile: 'release', ...settings.system },
    cases: [{ implementation: 'quent', exporter: 'noop', event_shape: 'empty', threads: 2,
      num_batches: 5, batch_size: 2, num_warmup_batches: 1, batch_pause_interval_us: 10,
      preflight_call: true, total_call_count: 10, discarded_call_count: 10,
      thread_batch_elapsed_ns: [[2, 4, 6, 8, 60], [2, 4, 6, 8, 60]],
      average_ns_per_iteration: 8,
      ...settings.case }],
  };
}

test('batch averages and box plot values use one sample per batch across threads', () => {
  const item = report(1).cases[0];
  assert.deepEqual(batchAverages(item), [1, 2, 3, 4, 30]);
  assert.deepEqual(boxSummary(item), {
    low: 1, q1: 2, median: 3, q3: 4, high: 4, outliers: [30],
  });
});

test('outlier jitter is stable and stays within the drawn box', () => {
  const offset = jitterOffset('latest:noop:30:0', 40, 5);
  assert.equal(jitterOffset('latest:noop:30:0', 40, 5), offset);
  assert.ok(Math.abs(offset) <= 17.5);
  const offsets = new Set(Array.from({ length: 100 }, (_, index) => jitterOffset(`point:${index}`, 40, 5)));
  assert.ok(Math.min(...offsets) < -13);
  assert.ok(Math.max(...offsets) > 13);
});

test('event grouping and labels include the implementation', () => {
  const value = report(1);
  const other = structuredClone(value.cases[0]);
  other.implementation = 'other-framework';
  other.exporter = null;
  value.cases.push(other);
  assert.equal(validateReport(value), value);
  assert.deepEqual(payloadGroups(value), [{ name: 'empty', threads: [2] }]);
  assert.equal(caseLabel(value.cases[0]), 'Quent / noop');
  assert.equal(caseLabel(other), 'other-framework / default');
});

test('language selection separates cases and accepts legacy Rust reports', () => {
  const value = report(1);
  const cpp = { ...value.cases[0], language: 'cpp' };
  const python = { ...value.cases[0], language: 'python', implementation: 'python-framework', event_shape: 'u8' };
  value.cases.push(cpp, python);
  assert.equal(validateReport(value), value);
  assert.deepEqual(availableLanguages(value), ['rust', 'cpp', 'python']);
  assert.deepEqual(payloadGroups(value, 'rust'), [{ name: 'empty', threads: [2] }]);
  assert.deepEqual(payloadGroups(value, 'cpp'), [{ name: 'empty', threads: [2] }]);
  assert.deepEqual(payloadGroups(value, 'cpp', false), []);
  assert.deepEqual(payloadGroups(value, 'python'), [{ name: 'u8', threads: [2] }]);
  assert.equal(caseLabel({ ...cpp, event_shape: null, exporter: null }), 'C++ / empty loop');
});

test('discarded cases can be filtered and marked', () => {
  const value = report(1);
  const current = { ...value.cases[0], implementation: 'current', exporter: 'file',
    discarded_call_count: 3 };
  const counted = { ...value.cases[0], implementation: 'counted', exporter: 'file',
    discarded_call_count: null, record_counts: { attempted: 10, observed: 7 } };
  const retained = { ...value.cases[0], implementation: 'retained', exporter: 'file',
    event_shape: 'u8', discarded_call_count: 0 };
  value.cases.push(current, counted, retained);
  assert.equal(discardedCallCount(value.cases[0]), 10);
  assert.equal(discardedCallCount(current), 3);
  assert.equal(discardedCallCount(counted), 3);
  assert.equal(discardedCallCount({ ...counted, discarded_call_count: 0 }), 0);
  assert.equal(caseAxisLabel(current), '⚠ current / file');
  assert.equal(caseAxisLabel(counted), '⚠ counted / file');
  assert.equal(caseAxisLabel(value.cases[0]), '⚠ Quent / noop');
  assert.deepEqual(payloadGroups(value, 'rust', true, false), [{ name: 'u8', threads: [2] }]);
  assert.deepEqual(payloadGroups(value, 'rust', false, false), [{ name: 'u8', threads: [2] }]);
});

test('fully discarded cases receive noop treatment', () => {
  const value = report(1);
  const adapterNoop = { ...value.cases[0], implementation: 'fastrace', exporter: 'adapter-noop',
    discarded_call_count: 10 };
  const discardedFile = { ...value.cases[0], implementation: 'slog-async', exporter: 'file',
    discarded_call_count: 2 };
  const partialNoop = { ...adapterNoop, discarded_call_count: 5 };
  const fullyDiscardedFile = { ...discardedFile, discarded_call_count: 10 };
  value.cases.push(adapterNoop, discardedFile);
  assert.equal(isNoopCase(adapterNoop), true);
  assert.equal(isNoopCase(partialNoop), false);
  assert.equal(isNoopCase(discardedFile), false);
  assert.equal(isNoopCase(fullyDiscardedFile), true);
  assert.deepEqual(payloadGroups(value, 'rust', false).map((group) => group.name), ['empty']);
  assert.equal(payloadGroups({ ...value, cases: [value.cases[0], adapterNoop] }, 'rust', false).length, 0);
});

test('cases sort by average and framework colors stay stable', () => {
  const slow = report(1).cases[0];
  const fast = { ...slow, implementation: 'other-framework', average_ns_per_iteration: 4 };
  const tie = { ...slow, exporter: 'other', average_ns_per_iteration: 8 };
  assert.deepEqual(sortCasesByAverage([slow, tie, fast]), [fast, slow, tie]);
  const names = ['quent', 'fastrace', 'minitrace', 'slog-async', 'spdlog-rs-async',
    'ticklog', 'tracing', 'opentelemetry', 'empty-loop-rs',
    ...Array.from({ length: 30 }, (_, index) => `framework-${index}`)];
  const colors = frameworkColors(names);
  assert.equal(new Set(colors.values()).size, names.length);
  assert.equal(frameworkColor('quent', colors), '#76B900');
  assert.equal(frameworkColor('quent', colors, undefined, 0), 'hsl(82, 0%, 36%)');
  assert.equal(frameworkColor('quent', colors), frameworkColor(slow.implementation, colors));
  assert.notEqual(frameworkColor('quent', colors), frameworkColor('fastrace', colors));
  assert.deepEqual([...frameworkColors([...names].reverse())], [...colors]);
});

test('newest report is selected regardless of hardware and settings', () => {
  const latest = report(4);
  const wrongMachine = report(3, { system: { cpu_model: 'other CPU' } });
  const wrongSettings = report(2, { case: { batch_size: 3 } });
  assert.equal(selectLatestReport([
    { report: latest }, { report: wrongMachine }, { report: wrongSettings },
  ]).report, latest);
  assert.equal(selectLatestReport([]), null);
});

test('incomplete report is skipped until it is valid', async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), 'quent-bench-plot-'));
  try {
    await writeFile(path.join(directory, 'old.json'), JSON.stringify(report(1)));
    await writeFile(path.join(directory, 'new.json'), '{');
    const warnings = [];
    assert.equal(selectLatestReport(await readReports(directory, (warning) => warnings.push(warning))).name, 'old.json');
    assert.equal(warnings.length, 1);
    await writeFile(path.join(directory, 'new.json'), JSON.stringify(report(2)));
    assert.equal(selectLatestReport(await readReports(directory)).name, 'new.json');
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('invalid timing dimensions and duplicate cases are rejected', () => {
  const invalid = report(1);
  invalid.cases[0].thread_batch_elapsed_ns[0].pop();
  assert.throws(() => validateReport(invalid), /invalid case/);
  const duplicate = report(1);
  duplicate.cases.push(structuredClone(duplicate.cases[0]));
  assert.throws(() => validateReport(duplicate), /duplicate case/);
  const invalidLanguage = report(1);
  invalidLanguage.cases[0].language = 'unknown';
  assert.throws(() => validateReport(invalidLanguage), /invalid case/);
});
