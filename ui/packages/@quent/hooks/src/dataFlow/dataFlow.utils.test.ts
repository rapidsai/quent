// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import type { DataFlowTimelineBinned, FsmTypeDecl, QuantitySpec } from '@quent/utils';
import {
  buildDataFlowMeta,
  computeWindowMax,
  extractBinConfig,
  extractDataFlowFrame,
  fitDataFlowSegmentLabel,
  formatDataFlowValue,
  formatDataFlowValueCompact,
  isDataFlowAvailable,
  normalizeDataFlowResponse,
  resolveDataFlowDimensions,
  resolveDataFlowLabelMeasure,
  resolveDataFlowMeasure,
  resolveDataFlowStates,
  resolveDataFlowWindow,
  timeToBinIndex,
  type DataFlowBinConfig,
} from './dataFlow.utils';

const NUM_BINS = 4;

/** 4 bins over [0, 8) seconds; two dimension keys; two measures. */
function makeBinned(operators: DataFlowTimelineBinned['operators'] = {}): DataFlowTimelineBinned {
  return {
    config: {
      span: { start: 0, end: 8 },
      bin_duration: 2,
      num_bins: BigInt(NUM_BINS),
    },
    decl: {
      entity_type_name: 'Task',
      dimension_name: 'Data location',
      dimension_keys: [
        { key: 'memory', display_name: 'Memory' },
        { key: 'filesystem', display_name: 'Filesystem' },
      ],
      measures: [
        { name: 'tasks', display_name: 'Tasks', quantity: 'unit', kind: 'Occupancy' },
        { name: 'bytes', display_name: 'Bytes', quantity: 'capacity_bytes', kind: 'Occupancy' },
      ],
    },
    operators,
  };
}

const OPERATORS: DataFlowTimelineBinned['operators'] = {
  'op-1': {
    values: {
      tasks: {
        queueing: {
          memory: [1, 2, 0, 0],
          // "filesystem" absent for queueing => zeros
        },
        computing: {
          memory: [0, 1, 3, 0],
          filesystem: [0, 0, 2, 0],
        },
      },
      bytes: {
        computing: {
          memory: [0, 0, 100, 0],
        },
      },
    },
  },
  'op-2': {
    values: {
      tasks: {
        queueing: {
          filesystem: [0, 4, 0, 0],
        },
      },
    },
  },
};

const FSM_TYPE: FsmTypeDecl = {
  name: 'Task',
  states: [
    { name: 'queueing', usages: [] },
    { name: 'allocating', usages: [] },
    { name: 'computing', usages: [] },
  ],
  transitions: [],
};

const UNIT_SPEC: QuantitySpec = {
  symbol: '',
  singular: 'task',
  plural: 'tasks',
  occupancy_prefix: 'None',
  rate_prefix: 'None',
};

const BYTES_SPEC: QuantitySpec = {
  symbol: 'B',
  singular: 'byte',
  plural: 'bytes',
  occupancy_prefix: 'Iec',
  rate_prefix: 'Si',
};

const BIN: DataFlowBinConfig = { startS: 0, endS: 8, binDurationS: 2, numBins: NUM_BINS };

describe('normalizeDataFlowResponse', () => {
  it('returns null for "Unsupported"', () => {
    expect(normalizeDataFlowResponse('Unsupported')).toBeNull();
  });

  it('returns null for null/undefined', () => {
    expect(normalizeDataFlowResponse(null)).toBeNull();
    expect(normalizeDataFlowResponse(undefined)).toBeNull();
  });

  it('unwraps the Binned variant', () => {
    const binned = makeBinned(OPERATORS);
    expect(normalizeDataFlowResponse({ Binned: binned })).toBe(binned);
  });
});

describe('isDataFlowAvailable', () => {
  it('is false for "Unsupported"', () => {
    expect(isDataFlowAvailable('Unsupported')).toBe(false);
  });

  it('is false for an empty operators map', () => {
    expect(isDataFlowAvailable({ Binned: makeBinned({}) })).toBe(false);
  });

  it('is true for a non-empty Binned response', () => {
    expect(isDataFlowAvailable({ Binned: makeBinned(OPERATORS) })).toBe(true);
  });
});

describe('resolveDataFlowWindow', () => {
  it('uses the zoom range when valid (end > start)', () => {
    expect(resolveDataFlowWindow({ start: 1, end: 3 }, 10)).toEqual({ start: 1, end: 3 });
  });

  it('falls back to [0, duration] for an unset zoom range', () => {
    expect(resolveDataFlowWindow({ start: 0, end: 0 }, 10)).toEqual({ start: 0, end: 10 });
  });

  it('falls back to [0, duration] for an inverted zoom range', () => {
    expect(resolveDataFlowWindow({ start: 5, end: 2 }, 10)).toEqual({ start: 0, end: 10 });
  });

  it('falls back to [0, duration] when zoom is null', () => {
    expect(resolveDataFlowWindow(null, 7)).toEqual({ start: 0, end: 7 });
  });
});

describe('timeToBinIndex', () => {
  it('maps times inside the window to their bin', () => {
    expect(timeToBinIndex(0, BIN)).toBe(0);
    expect(timeToBinIndex(1.9, BIN)).toBe(0);
    expect(timeToBinIndex(2, BIN)).toBe(1);
    expect(timeToBinIndex(7.5, BIN)).toBe(3);
  });

  it('clamps times before the window start to bin 0', () => {
    expect(timeToBinIndex(-5, BIN)).toBe(0);
  });

  it('clamps times at/after the window end to the last bin', () => {
    expect(timeToBinIndex(8, BIN)).toBe(NUM_BINS - 1);
    expect(timeToBinIndex(100, BIN)).toBe(NUM_BINS - 1);
  });

  it('returns 0 for degenerate bin configs', () => {
    expect(timeToBinIndex(3, { startS: 0, endS: 0, binDurationS: 0, numBins: 0 })).toBe(0);
  });
});

describe('extractBinConfig', () => {
  it('converts num_bins (bigint) to a number', () => {
    const bin = extractBinConfig(makeBinned(OPERATORS));
    expect(bin).toEqual({ startS: 0, endS: 8, binDurationS: 2, numBins: NUM_BINS });
    expect(typeof bin.numBins).toBe('number');
  });
});

describe('resolveDataFlowStates', () => {
  it('orders states per the FSM declaration, filtered to states present', () => {
    // Declared order: queueing, allocating, computing — allocating absent from data.
    expect(resolveDataFlowStates(makeBinned(OPERATORS), FSM_TYPE)).toEqual([
      'queueing',
      'computing',
    ]);
  });

  it('falls back to sorted data keys when the declaration is missing', () => {
    expect(resolveDataFlowStates(makeBinned(OPERATORS), null)).toEqual(['computing', 'queueing']);
  });

  it('appends undeclared data states after the declared ones, sorted', () => {
    const withExtra = makeBinned({
      'op-1': {
        values: {
          tasks: {
            zz_custom: { memory: [1, 0, 0, 0] },
            queueing: { memory: [1, 0, 0, 0] },
          },
        },
      },
    });
    expect(resolveDataFlowStates(withExtra, FSM_TYPE)).toEqual(['queueing', 'zz_custom']);
  });
});

describe('resolveDataFlowDimensions', () => {
  const KEYS = ['memory', 'filesystem'];

  it('resolves null to all declared keys', () => {
    expect([...resolveDataFlowDimensions(null, KEYS)]).toEqual(KEYS);
  });

  it('resolves an empty selection to all declared keys', () => {
    expect([...resolveDataFlowDimensions(new Set(), KEYS)]).toEqual(KEYS);
  });

  it('resolves a fully-stale selection (no declared keys) to all', () => {
    expect([...resolveDataFlowDimensions(new Set(['GPU-0', 'HOST']), KEYS)]).toEqual(KEYS);
  });

  it('keeps a valid subset, dropping unknown keys', () => {
    expect([...resolveDataFlowDimensions(new Set(['filesystem', 'nope']), KEYS)]).toEqual([
      'filesystem',
    ]);
  });
});

describe('computeWindowMax', () => {
  it('returns the max operator total across all bins', () => {
    // op-1 totals per bin: [1, 3, 5, 0]; op-2 totals per bin: [0, 4, 0, 0].
    expect(computeWindowMax(makeBinned(OPERATORS), 'tasks')).toBe(5);
  });

  it('treats absent measures as zero', () => {
    expect(computeWindowMax(makeBinned(OPERATORS), 'nope')).toBe(0);
  });

  it('is per-measure', () => {
    expect(computeWindowMax(makeBinned(OPERATORS), 'bytes')).toBe(100);
  });

  it('restricts to the selected dimension keys', () => {
    // filesystem only — op-1: [0, 0, 2, 0]; op-2: [0, 4, 0, 0].
    expect(computeWindowMax(makeBinned(OPERATORS), 'tasks', new Set(['filesystem']))).toBe(4);
    // memory only — op-1: [1, 3, 3, 0]; op-2 has no memory data.
    expect(computeWindowMax(makeBinned(OPERATORS), 'tasks', new Set(['memory']))).toBe(3);
  });

  it('treats a null/empty selection as all keys', () => {
    expect(computeWindowMax(makeBinned(OPERATORS), 'tasks', null)).toBe(5);
    expect(computeWindowMax(makeBinned(OPERATORS), 'tasks', new Set())).toBe(5);
  });
});

describe('extractDataFlowFrame', () => {
  const binned = makeBinned(OPERATORS);
  const stateNames = ['queueing', 'computing'];

  it('extracts totals, byState, byDimension and matrix at a bin', () => {
    const frame = extractDataFlowFrame(binned, stateNames, 'tasks', 2, 5);
    expect(frame.binIndex).toBe(2);
    expect(frame.timeS).toBe(4); // bin start: 0 + 2 * 2s
    expect(frame.measure).toBe('tasks');
    expect(frame.maxTotal).toBe(5);

    const op1 = frame.perOperator.get('op-1');
    expect(op1).toBeDefined();
    expect(op1!.total).toBe(5);
    expect(op1!.byState).toEqual([0, 5]); // queueing 0, computing 3+2
    expect(op1!.byDimension).toEqual([3, 2]); // memory, filesystem
    expect(op1!.matrix).toEqual([
      [0, 0],
      [3, 2],
    ]);
  });

  it('reads missing states/dimension keys as zero', () => {
    const frame = extractDataFlowFrame(binned, stateNames, 'tasks', 1, 5);
    const op2 = frame.perOperator.get('op-2');
    // op-2 only has queueing/filesystem — everything else is zero.
    expect(op2!.matrix).toEqual([
      [0, 4],
      [0, 0],
    ]);
    expect(op2!.byDimension).toEqual([0, 4]);
  });

  it('omits operators with an all-zero distribution at the bin', () => {
    const frame = extractDataFlowFrame(binned, stateNames, 'tasks', 3, 5);
    expect(frame.perOperator.size).toBe(0);
  });

  it('omits operators without the requested measure', () => {
    const frame = extractDataFlowFrame(binned, stateNames, 'bytes', 2, 100);
    expect(frame.perOperator.has('op-2')).toBe(false);
    expect(frame.perOperator.get('op-1')!.total).toBe(100);
  });

  it('clamps the bin index into range', () => {
    expect(extractDataFlowFrame(binned, stateNames, 'tasks', -3, 5).binIndex).toBe(0);
    expect(extractDataFlowFrame(binned, stateNames, 'tasks', 99, 5).binIndex).toBe(NUM_BINS - 1);
  });

  it('exposes per-operator totals for EVERY declared measure at the bin', () => {
    // Bin 2: op-1 tasks 3+2, bytes 100; op-2 all-zero (omitted entirely).
    const frame = extractDataFlowFrame(binned, stateNames, 'tasks', 2, 5);
    expect(frame.totalsByMeasure.get('op-1')).toEqual({ tasks: 5, bytes: 100 });
    expect(frame.totalsByMeasure.has('op-2')).toBe(false);
  });

  it('omits zero measures from the totals record', () => {
    // Bin 1: op-1 bytes are zero — only tasks appears.
    const frame = extractDataFlowFrame(binned, stateNames, 'tasks', 1, 5);
    expect(frame.totalsByMeasure.get('op-1')).toEqual({ tasks: 3 });
    expect(frame.totalsByMeasure.get('op-2')).toEqual({ tasks: 4 });
  });

  it('computes totalsByMeasure independently of the selected measure', () => {
    // Selected measure "bytes" has no data at bin 1, so perOperator is empty,
    // but the tasks totals are still exposed.
    const frame = extractDataFlowFrame(binned, stateNames, 'bytes', 1, 100);
    expect(frame.perOperator.size).toBe(0);
    expect(frame.totalsByMeasure.get('op-1')).toEqual({ tasks: 3 });
    expect(frame.totalsByMeasure.get('op-2')).toEqual({ tasks: 4 });
  });

  it('has an empty totalsByMeasure at an all-zero bin', () => {
    const frame = extractDataFlowFrame(binned, stateNames, 'tasks', 3, 5);
    expect(frame.totalsByMeasure.size).toBe(0);
  });

  it('aliases the label arrays to the bar arrays when no label measure is set', () => {
    const frame = extractDataFlowFrame(binned, stateNames, 'tasks', 2, 5);
    const op1 = frame.perOperator.get('op-1')!;
    expect(frame.labelMeasure).toBe('tasks');
    expect(op1.labelByState).toBe(op1.byState);
    expect(op1.labelByDimension).toBe(op1.byDimension);
  });

  it('computes label sums for an independent label measure, widths untouched', () => {
    const frame = extractDataFlowFrame(binned, stateNames, 'tasks', 2, 5, {
      labelMeasure: 'bytes',
    });
    expect(frame.labelMeasure).toBe('bytes');
    const op1 = frame.perOperator.get('op-1')!;
    // Bar-measure sums (segment widths) are unchanged...
    expect(op1.byState).toEqual([0, 5]);
    expect(op1.byDimension).toEqual([3, 2]);
    // ...while labels reflect bytes: computing/memory 100 at bin 2.
    expect(op1.labelByState).toEqual([0, 100]);
    expect(op1.labelByDimension).toEqual([100, 0]);
    // op-2 has no bytes data at all: label sums read as zero.
    const op2 = frame.perOperator.get('op-2');
    expect(op2).toBeUndefined(); // all-zero tasks at bin 2 — omitted anyway
  });

  it('reads label sums as zero for operators without the label measure', () => {
    const frame = extractDataFlowFrame(binned, stateNames, 'tasks', 1, 5, {
      labelMeasure: 'bytes',
    });
    const op2 = frame.perOperator.get('op-2')!;
    expect(op2.byState).toEqual([4, 0]);
    expect(op2.labelByState).toEqual([0, 0]);
    expect(op2.labelByDimension).toEqual([0, 0]);
  });

  it('filters every per-operator value to the selected dimensions', () => {
    const frame = extractDataFlowFrame(binned, stateNames, 'tasks', 2, 3, {
      selectedDimensions: new Set(['memory']),
    });
    const op1 = frame.perOperator.get('op-1')!;
    // Unselected filesystem column reads as zero everywhere.
    expect(op1.total).toBe(3);
    expect(op1.byState).toEqual([0, 3]);
    expect(op1.byDimension).toEqual([3, 0]);
    expect(op1.matrix).toEqual([
      [0, 0],
      [3, 0],
    ]);
    expect(frame.totalsByMeasure.get('op-1')).toEqual({ tasks: 3, bytes: 100 });
    // op-2 only has filesystem data — omitted entirely under this selection.
    expect(frame.perOperator.has('op-2')).toBe(false);
    expect(frame.totalsByMeasure.has('op-2')).toBe(false);
  });

  it('applies the dimension selection to label sums too', () => {
    const frame = extractDataFlowFrame(binned, stateNames, 'tasks', 2, 5, {
      labelMeasure: 'bytes',
      selectedDimensions: new Set(['filesystem']),
    });
    const op1 = frame.perOperator.get('op-1')!;
    // tasks/filesystem keeps op-1 visible, but bytes only exist in memory.
    expect(op1.byState).toEqual([0, 2]);
    expect(op1.labelByState).toEqual([0, 0]);
  });
});

describe('buildDataFlowMeta', () => {
  it('builds decl-driven meta with per-measure window max', () => {
    const meta = buildDataFlowMeta(makeBinned(OPERATORS), { Task: FSM_TYPE }, { unit: UNIT_SPEC });
    expect(meta.fsmType).toBe(FSM_TYPE);
    expect(meta.stateNames).toEqual(['queueing', 'computing']);
    expect(meta.bin.numBins).toBe(NUM_BINS);
    expect(meta.windowMax).toEqual({ tasks: 5, bytes: 100 });
    expect([...meta.dimensionSelection]).toEqual(['memory', 'filesystem']);
    expect(meta.quantitySpecs.unit).toBe(UNIT_SPEC);
  });

  it('tolerates a missing FSM declaration', () => {
    const meta = buildDataFlowMeta(makeBinned(OPERATORS), {}, undefined);
    expect(meta.fsmType).toBeNull();
    expect(meta.stateNames).toEqual(['computing', 'queueing']);
  });

  it('recomputes windowMax over the dimension selection', () => {
    const meta = buildDataFlowMeta(
      makeBinned(OPERATORS),
      { Task: FSM_TYPE },
      { unit: UNIT_SPEC },
      new Set(['filesystem'])
    );
    expect([...meta.dimensionSelection]).toEqual(['filesystem']);
    // tasks/filesystem maxes at 4 (op-2, bin 1); bytes live only in memory.
    expect(meta.windowMax).toEqual({ tasks: 4, bytes: 0 });
  });
});

describe('resolveDataFlowMeasure', () => {
  const decl = makeBinned().decl;

  it('keeps the selected measure when declared', () => {
    expect(resolveDataFlowMeasure('bytes', decl)).toBe('bytes');
  });

  it('falls back to the first declared measure when the selection is unknown', () => {
    expect(resolveDataFlowMeasure('nope', decl)).toBe('tasks');
    expect(resolveDataFlowMeasure(null, decl)).toBe('tasks');
  });

  it('returns null when no measures are declared', () => {
    expect(resolveDataFlowMeasure(null, { ...decl, measures: [] })).toBeNull();
  });
});

describe('resolveDataFlowLabelMeasure', () => {
  const decl = makeBinned().decl;

  it('keeps the selected label measure when declared', () => {
    expect(resolveDataFlowLabelMeasure('bytes', decl, 'tasks')).toBe('bytes');
  });

  it('follows the bar measure for null or unknown selections', () => {
    expect(resolveDataFlowLabelMeasure(null, decl, 'tasks')).toBe('tasks');
    expect(resolveDataFlowLabelMeasure('nope', decl, 'bytes')).toBe('bytes');
  });
});

describe('formatDataFlowValue', () => {
  const meta = buildDataFlowMeta(makeBinned(OPERATORS), { Task: FSM_TYPE }, { unit: UNIT_SPEC });

  it('formats via the measure quantity spec with ~1 decimal', () => {
    expect(formatDataFlowValue(2.5, 'tasks', meta)).toBe('2.5');
  });

  it('falls back to a plain fixed-point value without a spec', () => {
    // "bytes" quantity ("capacity_bytes") has no spec in this fixture.
    expect(formatDataFlowValue(3, 'bytes', meta)).toBe('3.0');
  });
});

describe('formatDataFlowValueCompact', () => {
  const meta = buildDataFlowMeta(
    makeBinned(OPERATORS),
    { Task: FSM_TYPE },
    { unit: UNIT_SPEC, capacity_bytes: BYTES_SPEC }
  );

  it('keeps one decimal below 10 and drops trailing .0', () => {
    expect(formatDataFlowValueCompact(3.2, 'tasks', meta)).toBe('3.2');
    expect(formatDataFlowValueCompact(2, 'tasks', meta)).toBe('2');
  });

  it('rounds to integers from 10 up (2-3 significant digits)', () => {
    expect(formatDataFlowValueCompact(45.3, 'tasks', meta)).toBe('45');
    expect(formatDataFlowValueCompact(482.4, 'tasks', meta)).toBe('482');
  });

  it('uses the IEC prefix + symbol without a space for bytes', () => {
    expect(formatDataFlowValueCompact(47185920, 'bytes', meta)).toBe('45MiB');
    expect(formatDataFlowValueCompact(1536, 'bytes', meta)).toBe('1.5KiB');
    expect(formatDataFlowValueCompact(100, 'bytes', meta)).toBe('100B');
  });

  it('falls back to a plain compact number for unknown measures', () => {
    expect(formatDataFlowValueCompact(7, 'nope', meta)).toBe('7');
  });
});

describe('fitDataFlowSegmentLabel', () => {
  const meta = buildDataFlowMeta(
    makeBinned(OPERATORS),
    { Task: FSM_TYPE },
    { unit: UNIT_SPEC, capacity_bytes: BYTES_SPEC }
  );
  const TRACK = 168;

  it('returns the compact label when the segment is wide enough', () => {
    // Full-width segment: 168px >= 1 char * 6px + 4px.
    expect(fitDataFlowSegmentLabel(5, 5, 'tasks', meta, TRACK)).toBe('5');
    expect(fitDataFlowSegmentLabel(100, 100, 'bytes', meta, TRACK)).toBe('100B');
  });

  it('hides the label when the segment is too narrow', () => {
    // 1/1000 of the track = 0.168px — far below the 10px needed for "1".
    expect(fitDataFlowSegmentLabel(1, 1000, 'tasks', meta, TRACK)).toBeNull();
  });

  it('gates exactly at charPx * length + padding', () => {
    // Label "5" needs 1 * 6 + 4 = 10px. Segment px = (5 / maxTotal) * 168.
    expect(fitDataFlowSegmentLabel(5, 84, 'tasks', meta, TRACK)).toBe('5'); // 10px
    expect(fitDataFlowSegmentLabel(5, 85, 'tasks', meta, TRACK)).toBeNull(); // ~9.88px
  });

  it('returns null for degenerate inputs', () => {
    expect(fitDataFlowSegmentLabel(0, 5, 'tasks', meta, TRACK)).toBeNull();
    expect(fitDataFlowSegmentLabel(5, 0, 'tasks', meta, TRACK)).toBeNull();
    expect(fitDataFlowSegmentLabel(5, 5, 'tasks', meta, 0)).toBeNull();
  });

  it('renders the label-measure text while the width stays on the bar measure', () => {
    expect(
      fitDataFlowSegmentLabel(5, 5, 'tasks', meta, TRACK, { value: 47185920, measure: 'bytes' })
    ).toBe('45MiB');
  });

  it('width-gates using the label text against the bar-measure segment width', () => {
    // Segment px = (5 / 42) * 168 = 20px: fits "5" (10px) but not the
    // 5-char "45MiB" (34px) from the label measure.
    expect(fitDataFlowSegmentLabel(5, 42, 'tasks', meta, TRACK)).toBe('5');
    expect(
      fitDataFlowSegmentLabel(5, 42, 'tasks', meta, TRACK, { value: 47185920, measure: 'bytes' })
    ).toBeNull();
  });

  it('hides the label when the label-measure value is zero', () => {
    expect(
      fitDataFlowSegmentLabel(5, 5, 'tasks', meta, TRACK, { value: 0, measure: 'bytes' })
    ).toBeNull();
  });
});
