// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import {
  formatDuration,
  formatDurationForWindow,
  formatDurationForAxisInterval,
  formatWithPrefix,
  formatNumber,
  formatNumberWithMaxFractionDigits,
  formatBytes,
  isBytesStat,
  isCountStat,
  inferFieldFormatter,
  formatQuantity,
} from './formatters';
import type { QuantitySpec } from './types/index';

// ---------------------------------------------------------------------------
// formatDuration
// ---------------------------------------------------------------------------

describe('formatDuration', () => {
  it('formats sub-microsecond values as nanoseconds', () => {
    expect(formatDuration(0.0000005)).toBe('0.50ns');
    expect(formatDuration(0.0009999)).toBe('999.90ns');
  });

  it('formats values in [0.001, 1) ms as microseconds', () => {
    expect(formatDuration(0.001)).toBe('1.00µs');
    expect(formatDuration(0.5)).toBe('500.00µs');
    expect(formatDuration(0.9999)).toBe('999.90µs');
  });

  it('formats values in [1, 1000) ms as milliseconds', () => {
    expect(formatDuration(1)).toBe('1.00ms');
    expect(formatDuration(250)).toBe('250.00ms');
    expect(formatDuration(999.9)).toBe('999.90ms');
  });

  it('formats values in [1000, 60000) ms as seconds', () => {
    expect(formatDuration(1000)).toBe('1.00s');
    expect(formatDuration(2500)).toBe('2.50s');
    expect(formatDuration(59999)).toBe('60.00s');
  });

  it('formats values in [60000, 3600000) ms as minutes', () => {
    expect(formatDuration(60000)).toBe('1.00min');
    expect(formatDuration(90000)).toBe('1.50min');
    expect(formatDuration(3599999)).toBe('60.00min');
  });

  it('formats values in [3600000, 86400000) ms as hours', () => {
    expect(formatDuration(3600000)).toBe('1.00h');
    expect(formatDuration(7200000)).toBe('2.00h');
  });

  it('formats values >= 86400000 ms as days', () => {
    expect(formatDuration(86400000)).toBe('1.00d');
    expect(formatDuration(172800000)).toBe('2.00d');
  });

  it('prepends a minus sign for negative values', () => {
    expect(formatDuration(-500)).toBe('-500.00ms');
    expect(formatDuration(-1000)).toBe('-1.00s');
    expect(formatDuration(-0.5)).toBe('-500.00µs');
  });

  it('formats zero as nanoseconds', () => {
    expect(formatDuration(0)).toBe('0.00ns');
  });

  it('respects the decimals parameter', () => {
    expect(formatDuration(500, 0)).toBe('500ms');
    expect(formatDuration(500, 1)).toBe('500.0ms');
    expect(formatDuration(500, 4)).toBe('500.0000ms');
  });
});

// ---------------------------------------------------------------------------
// formatDurationForWindow
// ---------------------------------------------------------------------------

describe('formatDurationForWindow', () => {
  it('uses 0 decimals when the window is much larger than the value unit', () => {
    // resolution = 10000/1000 = 10 ms; resolutionInUnit = 10 → decimals = 0
    expect(formatDurationForWindow(500, 10000)).toBe('500ms');
  });

  it('adds decimals as the window narrows', () => {
    // resolution = 100/1000 = 0.1 ms → decimals = 1
    expect(formatDurationForWindow(500, 100)).toBe('500.0ms');

    // resolution = 10/1000 = 0.01 ms → decimals = 2
    expect(formatDurationForWindow(500, 10)).toBe('500.00ms');

    // resolution = 1/1000 = 0.001 ms → decimals = 3
    expect(formatDurationForWindow(500, 1)).toBe('500.000ms');
  });

  it('still selects the correct unit for the value', () => {
    // windowMs=1000 → resolution=1ms, unitMs=1000 → ratio=0.001 → decimals=3
    expect(formatDurationForWindow(2000, 1000)).toBe('2.000s');
  });
});

// ---------------------------------------------------------------------------
// formatDurationForAxisInterval
// ---------------------------------------------------------------------------

describe('formatDurationForAxisInterval', () => {
  it('uses 0 decimals when the interval is >= the display unit', () => {
    // intervalMs = 100 ms, unitMs = 1 → intervalInUnit = 100 → decimals = 0
    expect(formatDurationForAxisInterval(500, 100)).toBe('500ms');
  });

  it('adds decimals as the interval shrinks below 1 unit', () => {
    // intervalMs = 0.1 ms → intervalInUnit = 0.1 → decimals = 1
    expect(formatDurationForAxisInterval(500, 0.1)).toBe('500.0ms');

    // intervalMs = 0.001 ms → intervalInUnit = 0.001 → decimals = 3
    expect(formatDurationForAxisInterval(500, 0.001)).toBe('500.000ms');
  });

  it('selects the correct unit based on the value', () => {
    // value 2000 ms → seconds range, interval 0.01 ms → 0.01/1000 = 1e-5 s → decimals = 5
    expect(formatDurationForAxisInterval(2000, 0.01)).toBe('2.00000s');
  });
});

// ---------------------------------------------------------------------------
// formatWithPrefix — None
// ---------------------------------------------------------------------------

describe('formatWithPrefix (None)', () => {
  it('formats with symbol and no prefix scaling', () => {
    expect(formatWithPrefix(42, 'Hz', 'None')).toBe('42.0 Hz');
    expect(formatWithPrefix(1e9, 'Hz', 'None')).toBe('1000000000.0 Hz');
  });

  it('formats without symbol', () => {
    expect(formatWithPrefix(42, '', 'None')).toBe('42.0');
  });

  it('handles negative values', () => {
    expect(formatWithPrefix(-42, 'Hz', 'None')).toBe('-42.0 Hz');
  });

  it('formats zero', () => {
    expect(formatWithPrefix(0, 'Hz', 'None')).toBe('0 Hz');
    expect(formatWithPrefix(0, '', 'None')).toBe('0');
  });

  it('respects decimals', () => {
    expect(formatWithPrefix(3.14159, 'Hz', 'None', 3)).toBe('3.142 Hz');
  });
});

// ---------------------------------------------------------------------------
// formatWithPrefix — Si (upward scaling)
// ---------------------------------------------------------------------------

describe('formatWithPrefix (Si, values >= 1)', () => {
  it('uses no prefix for values in [1, 1000)', () => {
    expect(formatWithPrefix(1, 'Hz', 'Si')).toBe('1.0 Hz');
    expect(formatWithPrefix(500, 'Hz', 'Si')).toBe('500.0 Hz');
  });

  it('uses k prefix for values in [1e3, 1e6)', () => {
    expect(formatWithPrefix(1000, 'Hz', 'Si')).toBe('1.0 kHz');
    expect(formatWithPrefix(1500, 'Hz', 'Si')).toBe('1.5 kHz');
  });

  it('uses M prefix for values in [1e6, 1e9)', () => {
    expect(formatWithPrefix(1e6, 'Hz', 'Si')).toBe('1.0 MHz');
  });

  it('uses G prefix for values in [1e9, 1e12)', () => {
    expect(formatWithPrefix(1e9, 'Hz', 'Si')).toBe('1.0 GHz');
  });

  it('uses T prefix for values in [1e12, 1e15)', () => {
    expect(formatWithPrefix(1e12, 'Hz', 'Si')).toBe('1.0 THz');
  });

  it('uses P prefix for values >= 1e15', () => {
    expect(formatWithPrefix(1e15, 'Hz', 'Si')).toBe('1.0 PHz');
  });

  it('handles negative values', () => {
    expect(formatWithPrefix(-1500, 'Hz', 'Si')).toBe('-1.5 kHz');
  });
});

// ---------------------------------------------------------------------------
// formatWithPrefix — Si (downward scaling for values < 1)
// ---------------------------------------------------------------------------

describe('formatWithPrefix (Si, values < 1)', () => {
  it('uses m prefix for values in [1e-3, 1)', () => {
    expect(formatWithPrefix(0.001, 'W', 'Si')).toBe('1.0 mW');
    expect(formatWithPrefix(0.5, 'W', 'Si')).toBe('500.0 mW');
  });

  it('uses µ prefix for values in [1e-6, 1e-3)', () => {
    expect(formatWithPrefix(1e-6, 'W', 'Si')).toBe('1.0 µW');
    expect(formatWithPrefix(5e-4, 'W', 'Si')).toBe('500.0 µW');
  });

  it('uses n prefix for values in [1e-9, 1e-6)', () => {
    expect(formatWithPrefix(1e-9, 'W', 'Si')).toBe('1.0 nW');
  });

  it('uses p prefix for values in [1e-12, 1e-9)', () => {
    expect(formatWithPrefix(1e-12, 'W', 'Si')).toBe('1.0 pW');
  });
});

// ---------------------------------------------------------------------------
// formatWithPrefix — IEC
// ---------------------------------------------------------------------------

describe('formatWithPrefix (Iec)', () => {
  it('uses no prefix for values in [1, 1024)', () => {
    expect(formatWithPrefix(1, 'B', 'Iec')).toBe('1.0 B');
    expect(formatWithPrefix(512, 'B', 'Iec')).toBe('512.0 B');
  });

  it('uses Ki prefix for values in [1024, 1024^2)', () => {
    expect(formatWithPrefix(1024, 'B', 'Iec')).toBe('1.0 KiB');
    expect(formatWithPrefix(1536, 'B', 'Iec')).toBe('1.5 KiB');
  });

  it('uses Mi prefix for values in [1024^2, 1024^3)', () => {
    expect(formatWithPrefix(1048576, 'B', 'Iec')).toBe('1.0 MiB');
  });

  it('uses Gi prefix for values in [1024^3, 1024^4)', () => {
    expect(formatWithPrefix(1073741824, 'B', 'Iec')).toBe('1.0 GiB');
  });

  it('uses Ti prefix for values in [1024^4, 1024^5)', () => {
    expect(formatWithPrefix(1099511627776, 'B', 'Iec')).toBe('1.0 TiB');
  });

  it('handles negative values', () => {
    expect(formatWithPrefix(-1024, 'B', 'Iec')).toBe('-1.0 KiB');
  });
});

// ---------------------------------------------------------------------------
// formatBytes
// ---------------------------------------------------------------------------

describe('formatBytes', () => {
  it('wraps formatWithPrefix with IEC and B symbol', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(1024)).toBe('1.0 KiB');
    expect(formatBytes(1048576)).toBe('1.0 MiB');
    expect(formatBytes(1073741824)).toBe('1.0 GiB');
  });

  it('respects the decimals parameter', () => {
    expect(formatBytes(1536, 2)).toBe('1.50 KiB');
  });
});

// ---------------------------------------------------------------------------
// formatNumber
// ---------------------------------------------------------------------------

describe('formatNumber', () => {
  it('formats integers with locale grouping separators', () => {
    // Assumes en-US locale (standard in CI)
    expect(formatNumber(0)).toBe('0');
    expect(formatNumber(999)).toBe('999');
    expect(formatNumber(1000)).toBe('1,000');
    expect(formatNumber(1234567)).toBe('1,234,567');
  });

  it('formats non-integers to 3 significant figures', () => {
    expect(formatNumber(1.23456)).toBe('1.23');
    expect(formatNumber(0.001234)).toBe('0.00123');
    expect(formatNumber(12345.6)).toBe('12,300');
  });
});

// ---------------------------------------------------------------------------
// formatNumberWithMaxFractionDigits
// ---------------------------------------------------------------------------

describe('formatNumberWithMaxFractionDigits', () => {
  it('formats integers with locale grouping separators', () => {
    expect(formatNumberWithMaxFractionDigits(42)).toBe('42');
    expect(formatNumberWithMaxFractionDigits(1000)).toBe('1,000');
  });

  it('rounds floats to the given maximum fraction digits (default 4)', () => {
    expect(formatNumberWithMaxFractionDigits(3.14159)).toBe('3.1416');
    expect(formatNumberWithMaxFractionDigits(0.00001)).toBe('0');
  });

  it('respects a custom maximumFractionDigits', () => {
    expect(formatNumberWithMaxFractionDigits(3.14159, 2)).toBe('3.14');
    expect(formatNumberWithMaxFractionDigits(3.14159, 0)).toBe('3');
  });
});

// ---------------------------------------------------------------------------
// isBytesStat
// ---------------------------------------------------------------------------

describe('isBytesStat', () => {
  it('returns true for names containing _bytes', () => {
    expect(isBytesStat('bytes_read')).toBe(true);
    expect(isBytesStat('total_bytes_written')).toBe(true);
    expect(isBytesStat('spill_bytes')).toBe(true);
  });

  it('returns true for names ending with _byte', () => {
    expect(isBytesStat('output_byte')).toBe(true);
  });

  it('returns true for names starting with bytes_', () => {
    expect(isBytesStat('bytes_produced')).toBe(true);
  });

  it('returns true for the bare name "bytes"', () => {
    expect(isBytesStat('bytes')).toBe(true);
  });

  it('returns false for unrelated names', () => {
    expect(isBytesStat('row_count')).toBe(false);
    expect(isBytesStat('elapsed_ns')).toBe(false);
    expect(isBytesStat('throughput_mbs')).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// isCountStat
// ---------------------------------------------------------------------------

describe('isCountStat', () => {
  it('returns true for names containing _rows', () => {
    expect(isCountStat('output_rows')).toBe(true);
    expect(isCountStat('input_rows_filtered')).toBe(true);
  });

  it('returns true for names ending with _row', () => {
    expect(isCountStat('processed_row')).toBe(true);
  });

  it('returns true for names starting with rows_', () => {
    expect(isCountStat('rows_produced')).toBe(true);
  });

  it('returns true for names containing _batches', () => {
    expect(isCountStat('output_batches')).toBe(true);
  });

  it('returns true for names ending with _batch', () => {
    expect(isCountStat('last_batch')).toBe(true);
  });

  it('returns true for names starting with batches_', () => {
    expect(isCountStat('batches_processed')).toBe(true);
  });

  it('returns false for unrelated names', () => {
    expect(isCountStat('elapsed_ns')).toBe(false);
    expect(isCountStat('bytes_read')).toBe(false);
    expect(isCountStat('hit_ratio')).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// inferFieldFormatter
// ---------------------------------------------------------------------------

describe('inferFieldFormatter', () => {
  it('formats _ns fields as duration (ns → ms conversion)', () => {
    const fmt = inferFieldFormatter('elapsed_ns');
    expect(fmt(1_000_000)).toBe('1.00ms'); // 1e6 ns = 1 ms
    expect(fmt(1_000_000_000)).toBe('1.00s'); // 1e9 ns = 1 s
  });

  it('formats byte fields with IEC byte units', () => {
    const fmt = inferFieldFormatter('spill_bytes');
    expect(fmt(1024)).toBe('1.00 KiB');
    expect(fmt(1048576)).toBe('1.00 MiB');
  });

  it('formats row/batch count fields with SI scaling', () => {
    const fmtRows = inferFieldFormatter('output_rows');
    expect(fmtRows(500)).toBe('500.00 ');
    expect(fmtRows(1500)).toBe('1.50 k');

    const fmtBatches = inferFieldFormatter('spill_batches');
    expect(fmtBatches(2000)).toBe('2.00 k');
  });

  it('formats _mbs fields as MB/s', () => {
    const fmt = inferFieldFormatter('throughput_mbs');
    expect(fmt(100)).toBe('100.0 MB/s');
    expect(fmt(0.5)).toBe('0.5 MB/s');
  });

  it('formats ratio/fraction/selectivity/rate fields as percentages', () => {
    expect(inferFieldFormatter('hit_ratio')(0.75)).toBe('75.0%');
    expect(inferFieldFormatter('null_fraction')(0.1)).toBe('10.0%');
    expect(inferFieldFormatter('false_positive_fpr')(0.05)).toBe('5.0%');
    expect(inferFieldFormatter('probe_selectivity')(0.333)).toBe('33.3%');
    expect(inferFieldFormatter('cache_rate')(0.99)).toBe('99.0%');
  });

  it('formats unrecognised fields as numbers with max 4 fraction digits', () => {
    expect(inferFieldFormatter('custom_stat')(42)).toBe('42');
    expect(inferFieldFormatter('custom_stat')(3.14159)).toBe('3.1416');
  });
});

// ---------------------------------------------------------------------------
// formatQuantity
// ---------------------------------------------------------------------------

describe('formatQuantity', () => {
  const bytesSpec: QuantitySpec = {
    symbol: 'B',
    singular: 'byte',
    plural: 'bytes',
    occupancy_prefix: 'Iec',
    rate_prefix: 'Si',
  };

  it('formats Occupancy using the occupancy_prefix and bare symbol', () => {
    expect(formatQuantity(1024, bytesSpec, 'Occupancy')).toBe('1.00 KiB');
    expect(formatQuantity(1073741824, bytesSpec, 'Occupancy')).toBe('1.00 GiB');
  });

  it('formats Rate using the rate_prefix and appends /s to the symbol', () => {
    expect(formatQuantity(1000, bytesSpec, 'Rate')).toBe('1.00 kB/s');
    expect(formatQuantity(1e6, bytesSpec, 'Rate')).toBe('1.00 MB/s');
  });

  it('respects the decimals parameter', () => {
    expect(formatQuantity(1536, bytesSpec, 'Occupancy', 1)).toBe('1.5 KiB');
  });

  it('handles a None-prefix spec', () => {
    const countSpec: QuantitySpec = {
      symbol: 'rows',
      singular: 'row',
      plural: 'rows',
      occupancy_prefix: 'None',
      rate_prefix: 'None',
    };
    expect(formatQuantity(42, countSpec, 'Occupancy', 0)).toBe('42 rows');
    expect(formatQuantity(100, countSpec, 'Rate', 1)).toBe('100.0 rows/s');
  });
});
