// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from 'vitest';
import {
  withOpacity,
  isLightColor,
  getDeterministicColor,
  buildDeterministicColorMap,
  extendDeterministicColorMap,
  createDeterministicColorResolver,
  continuousColor,
  getLegendGradientStops,
} from './colors';

// ---- withOpacity -----------------------------------------------------------

describe('withOpacity', () => {
  it('appends FF for opacity 1', () => {
    expect(withOpacity('#0072B2', 1)).toBe('#0072B2FF');
  });

  it('appends 00 for opacity 0', () => {
    expect(withOpacity('#0072B2', 0)).toBe('#0072B200');
  });

  it('appends the correct two-digit hex for opacity 0.5', () => {
    // Math.round(0.5 * 255) = Math.round(127.5) = 128 = 0x80
    expect(withOpacity('#0072B2', 0.5)).toBe('#0072B280');
  });

  it('clamps opacity above 1 to FF', () => {
    expect(withOpacity('#ffffff', 2)).toBe('#ffffffFF');
  });

  it('clamps opacity below 0 to 00', () => {
    expect(withOpacity('#ffffff', -1)).toBe('#ffffff00');
  });

  it('pads single-digit alpha values to two chars', () => {
    // Math.round(0.02 * 255) = Math.round(5.1) = 5 = 0x05
    expect(withOpacity('#000000', 0.02)).toBe('#00000005');
  });
});

// ---- isLightColor ----------------------------------------------------------

describe('isLightColor', () => {
  it('returns true for white', () => {
    expect(isLightColor('#ffffff')).toBe(true);
  });

  it('returns false for black', () => {
    expect(isLightColor('#000000')).toBe(false);
  });

  it('returns true for a bright yellow (#F0E442)', () => {
    expect(isLightColor('#F0E442')).toBe(true);
  });

  it('returns false for a dark blue (#0072B2)', () => {
    expect(isLightColor('#0072B2')).toBe(false);
  });

  it('returns false for a medium dark teal (#009E73)', () => {
    // r=0, g=158/255=0.620, b=115/255=0.451
    // luminance = 0 + 0.587*0.620 + 0.114*0.451 ≈ 0.364 + 0.051 = 0.415 < 0.5
    expect(isLightColor('#009E73')).toBe(false);
  });
});

// ---- getDeterministicColor -------------------------------------------------

describe('getDeterministicColor', () => {
  it('returns a hex color string', () => {
    expect(getDeterministicColor('Scan')).toMatch(/^#[0-9a-fA-F]{6}$/);
  });

  it('is deterministic for the same key', () => {
    expect(getDeterministicColor('Join')).toBe(getDeterministicColor('Join'));
  });

  it('is case-insensitive', () => {
    expect(getDeterministicColor('SCAN')).toBe(getDeterministicColor('scan'));
    expect(getDeterministicColor('Scan')).toBe(getDeterministicColor('scan'));
  });

  it('different common keys may have different colors', () => {
    const colors = new Set(['Scan', 'Join', 'Aggregate', 'Sort'].map(getDeterministicColor));
    expect(colors.size).toBeGreaterThan(1);
  });
});

describe('deterministic color maps', () => {
  it('normalizes and deterministically assigns primitive keys regardless of input order', () => {
    const first = buildDeterministicColorMap([' Scan ', 42, 7n]);
    const second = buildDeterministicColorMap([7n, 'scan', 42]);

    expect(first).toEqual(second);
    expect(first.has('scan')).toBe(true);
    expect(new Set(first.values()).size).toBe(first.size);
  });

  it('uses an explicit stable key for object values', () => {
    const values = [
      { id: 'beta', label: 'Second' },
      { id: 'alpha', label: 'First' },
    ];
    const map = buildDeterministicColorMap(values, value => value.id);
    const resolveColor = createDeterministicColorResolver(map);

    expect(resolveColor({ id: 'alpha' }, value => value.id)).toBe(map.get('alpha'));
  });

  it('falls back deterministically for keys absent from the precomputed map', () => {
    const resolveColor = createDeterministicColorResolver(buildDeterministicColorMap(['known']));

    expect(resolveColor('unknown')).toBe(getDeterministicColor('unknown'));
    expect(resolveColor(' Unknown ')).toBe(resolveColor('unknown'));
  });

  it('uses the supplied palette for maps and resolver fallbacks', () => {
    const palette = ['#111111', '#222222', '#333333'];
    const map = buildDeterministicColorMap(['known-a', 'known-b'], palette);
    const resolveColor = createDeterministicColorResolver(map, palette);

    expect([...map.values()].every(color => palette.includes(color))).toBe(true);
    expect(palette).toContain(resolveColor('unknown'));
  });

  it('rejects empty palettes', () => {
    expect(() => buildDeterministicColorMap(['known'], [])).toThrow(
      'Color palettes must contain at least one color'
    );
  });
});

// ---- buildDeterministicColorMap --------------------------------------------

describe('buildDeterministicColorMap', () => {
  it('returns an empty map for an empty input', () => {
    expect(buildDeterministicColorMap([])).toEqual(new Map());
  });

  it('returns a map entry for each unique key', () => {
    const map = buildDeterministicColorMap(['Scan', 'Join', 'Aggregate']);
    expect(map.size).toBe(3);
    expect(map.has('scan')).toBe(true);
    expect(map.has('join')).toBe(true);
    expect(map.has('aggregate')).toBe(true);
  });

  it('deduplicates case-insensitively', () => {
    const map = buildDeterministicColorMap(['Scan', 'SCAN', 'scan']);
    expect(map.size).toBe(1);
  });

  it('assigns distinct colors to distinct keys (up to palette size)', () => {
    const keys = ['alpha', 'beta', 'gamma', 'delta'];
    const map = buildDeterministicColorMap(keys);
    const colors = Array.from(map.values());
    const unique = new Set(colors);
    expect(unique.size).toBe(keys.length);
  });

  it('is deterministic: same input always produces the same map', () => {
    const m1 = buildDeterministicColorMap(['Scan', 'Join']);
    const m2 = buildDeterministicColorMap(['Scan', 'Join']);
    expect(m1.get('scan')).toBe(m2.get('scan'));
    expect(m1.get('join')).toBe(m2.get('join'));
  });
});

describe('extendDeterministicColorMap', () => {
  it('preserves existing assignments and avoids their colors for new keys', () => {
    const base = buildDeterministicColorMap(['declared-a', 'declared-b']);
    const extended = extendDeterministicColorMap(base, ['synthetic']);

    expect(extended.get('declared-a')).toBe(base.get('declared-a'));
    expect(extended.get('declared-b')).toBe(base.get('declared-b'));
    expect([...base.values()]).not.toContain(extended.get('synthetic'));
  });

  it('assigns additional values independently of input order', () => {
    const base = buildDeterministicColorMap(['declared']);

    expect(extendDeterministicColorMap(base, ['zeta', 'alpha'])).toEqual(
      extendDeterministicColorMap(base, ['alpha', 'zeta'])
    );
  });
});

// ---- continuousColor -------------------------------------------------------

describe('continuousColor', () => {
  it('returns the neutral color at t=0 (light mode, blue)', () => {
    // blendToColor(59, 130, 246, 0, [255, 255, 255]) → #ffffff
    expect(continuousColor(0, 'blue')).toBe('#ffffff');
  });

  it('returns the full color at t=1 (light mode, blue)', () => {
    // blendToColor(59, 130, 246, 1, [255, 255, 255]) → #3b82f6
    expect(continuousColor(1, 'blue')).toBe('#3b82f6');
  });

  it('clamps t below 0 to neutral', () => {
    expect(continuousColor(-1, 'blue')).toBe(continuousColor(0, 'blue'));
  });

  it('clamps t above 1 to the full color', () => {
    expect(continuousColor(2, 'blue')).toBe(continuousColor(1, 'blue'));
  });

  it('returns the correct teal color at t=1', () => {
    // blendToColor(20, 184, 166, 1) → #14b8a6
    expect(continuousColor(1, 'teal')).toBe('#14b8a6');
  });

  it('returns the correct purple color at t=1', () => {
    // blendToColor(168, 85, 247, 1) → #a855f7
    expect(continuousColor(1, 'purple')).toBe('#a855f7');
  });

  it('returns the correct orange color at t=1', () => {
    // blendToColor(249, 115, 22, 1) → #f97316
    expect(continuousColor(1, 'orange')).toBe('#f97316');
  });

  it('uses a darker neutral in dark mode', () => {
    const light = continuousColor(0, 'blue', false);
    const dark = continuousColor(0, 'blue', true);
    expect(light).not.toBe(dark);
  });

  it('viridis at t=0 returns the deep purple stop', () => {
    // VIRIDIS_STOPS[0] = [68,1,84] → #440154
    expect(continuousColor(0, 'viridis')).toBe('#440154');
  });

  it('viridis at t=1 returns the bright yellow stop', () => {
    // VIRIDIS_STOPS[4] = [253,231,37] → #fde725
    expect(continuousColor(1, 'viridis')).toBe('#fde725');
  });

  it('viridis at t=0.5 lands on the teal stop (exact midpoint)', () => {
    // scaled = 0.5*4 = 2.0, lo=2, hi=3, frac=0
    // VIRIDIS_STOPS[2] = [33,145,140] → #21918c
    expect(continuousColor(0.5, 'viridis')).toBe('#21918c');
  });
});

// ---- getLegendGradientStops ------------------------------------------------

describe('getLegendGradientStops', () => {
  it('returns exactly 2 stops for non-viridis palettes', () => {
    expect(getLegendGradientStops('blue')).toHaveLength(2);
    expect(getLegendGradientStops('teal')).toHaveLength(2);
    expect(getLegendGradientStops('purple')).toHaveLength(2);
    expect(getLegendGradientStops('orange')).toHaveLength(2);
  });

  it('first stop is t=0 and last stop is t=1 for a simple palette', () => {
    const stops = getLegendGradientStops('blue');
    expect(stops[0]).toBe(continuousColor(0, 'blue'));
    expect(stops[1]).toBe(continuousColor(1, 'blue'));
  });

  it('returns one stop per viridis color stop', () => {
    // VIRIDIS_STOPS has 5 entries
    expect(getLegendGradientStops('viridis')).toHaveLength(5);
  });

  it('viridis stops span from the dark purple to the bright yellow', () => {
    const stops = getLegendGradientStops('viridis');
    expect(stops[0]).toBe('#440154');
    expect(stops[stops.length - 1]).toBe('#fde725');
  });

  it('dark mode produces different stops than light mode', () => {
    const light = getLegendGradientStops('blue', false);
    const dark = getLegendGradientStops('blue', true);
    expect(light[0]).not.toBe(dark[0]);
  });
});
