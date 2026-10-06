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
  COLOR_PALETTES,
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

// ---- palette separation ----------------------------------------------------

type Matrix = readonly [
  readonly [number, number, number],
  readonly [number, number, number],
  readonly [number, number, number],
];

// Color blindness simulation matrices (Machado et al. 2009, full severity).
const PROTAN: Matrix = [
  [0.152286, 1.052583, -0.204868],
  [0.114503, 0.786281, 0.099216],
  [-0.003882, -0.048116, 1.051998],
];
const DEUTAN: Matrix = [
  [0.367322, 0.860646, -0.227968],
  [0.280085, 0.672501, 0.047413],
  [-0.01182, 0.04294, 0.968881],
];

function toLinear(hex: string): [number, number, number] {
  return [1, 3, 5].map(i => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  }) as [number, number, number];
}

function toOklab([r, g, b]: [number, number, number]): [number, number, number] {
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  return [
    0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  ];
}

function simulate(rgb: [number, number, number], matrix?: Matrix): [number, number, number] {
  if (!matrix) return rgb;
  const clamp = (v: number) => Math.min(1, Math.max(0, v));
  return matrix.map(row => clamp(row[0] * rgb[0] + row[1] * rgb[1] + row[2] * rgb[2])) as [
    number,
    number,
    number,
  ];
}

/** Distance between two colors in OKLab (x100), optionally as seen with color blindness. */
function colorDistance(a: string, b: string, matrix?: Matrix): number {
  const [l1, a1, b1] = toOklab(simulate(toLinear(a), matrix));
  const [l2, a2, b2] = toOklab(simulate(toLinear(b), matrix));
  return 100 * Math.hypot(l1 - l2, a1 - a2, b1 - b2);
}

describe('COLOR_PALETTES.deterministic', () => {
  const palette = COLOR_PALETTES.deterministic;
  // No two colors may look alike, with normal vision or red-green color blindness.
  const MIN_DISTANCE = 5.5;

  it('has no duplicate colors', () => {
    expect(new Set(palette).size).toBe(palette.length);
  });

  it.each([
    ['normal vision', undefined],
    ['protanopia', PROTAN],
    ['deuteranopia', DEUTAN],
  ] as const)('keeps every pair of colors apart under %s', (_name, matrix) => {
    for (let i = 0; i < palette.length; i++) {
      for (let j = i + 1; j < palette.length; j++) {
        expect(
          colorDistance(palette[i]!, palette[j]!, matrix),
          `${palette[i]} vs ${palette[j]}`
        ).toBeGreaterThanOrEqual(MIN_DISTANCE);
      }
    }
  });
});
