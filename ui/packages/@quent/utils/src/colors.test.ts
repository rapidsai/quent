// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect, beforeEach } from 'vitest';
import {
  PALETTES,
  getPalette,
  getActivePalette,
  setActivePalette,
  getColorForKey,
  assignColors,
  createCapacitiesColorFn,
  getColorByIndex,
  createFsmTypeColorFn,
  withOpacity,
  resetColorAssignments,
  darkenColor,
  isLightColor,
  getOperationTypeColor,
  buildOperatorColorMap,
  continuousColor,
  getLegendGradientStops,
} from './colors';

// Reset all mutable module state before each test to prevent cross-test bleed.
beforeEach(() => {
  setActivePalette('extended'); // restores default
  resetColorAssignments();
});

// ---- getPalette ------------------------------------------------------------

describe('getPalette', () => {
  it('returns the wong palette (flat array, theme-independent)', () => {
    expect(getPalette('wong')).toBe(PALETTES.wong);
    expect(getPalette('wong', 'dark')).toBe(PALETTES.wong);
  });

  it('returns the echarts palette (flat array, theme-independent)', () => {
    expect(getPalette('echarts')).toBe(PALETTES.echarts);
  });

  it('returns the extended light palette', () => {
    expect(getPalette('extended', 'light')).toBe(PALETTES.extended.light);
  });

  it('returns the extended dark palette', () => {
    expect(getPalette('extended', 'dark')).toBe(PALETTES.extended.dark);
  });

  it('defaults theme to light for extended', () => {
    expect(getPalette('extended')).toBe(getPalette('extended', 'light'));
  });
});

// ---- getActivePalette / setActivePalette -----------------------------------

describe('getActivePalette', () => {
  it('defaults to the extended light palette', () => {
    expect(getActivePalette('light')).toBe(PALETTES.extended.light);
  });

  it('defaults theme parameter to light', () => {
    expect(getActivePalette()).toEqual(getActivePalette('light'));
  });

  it('returns the extended dark palette when theme is dark', () => {
    expect(getActivePalette('dark')).toBe(PALETTES.extended.dark);
  });

  it('reflects a setActivePalette change', () => {
    setActivePalette('wong');
    expect(getActivePalette('light')).toBe(PALETTES.wong);
  });

  it('flat palettes (wong, echarts) ignore theme', () => {
    setActivePalette('wong');
    expect(getActivePalette('light')).toEqual(getActivePalette('dark'));
  });
});

describe('setActivePalette', () => {
  it('resets color assignments when switching palettes', () => {
    const colorBefore = getColorForKey('key', 'light');
    setActivePalette('wong'); // also resets assignments
    resetColorAssignments();
    setActivePalette('extended');
    resetColorAssignments();
    // After a full reset, the same key should get its hash-based index again
    const colorAfter = getColorForKey('key', 'light');
    expect(colorBefore).toBe(colorAfter);
  });
});

// ---- getColorByIndex -------------------------------------------------------

describe('getColorByIndex', () => {
  it('returns the first palette color at index 0', () => {
    expect(getColorByIndex(0, 'light')).toBe(PALETTES.extended.light[0]);
  });

  it('returns the correct color for an arbitrary in-range index', () => {
    expect(getColorByIndex(2, 'light')).toBe(PALETTES.extended.light[2]);
  });

  it('wraps around when the index exceeds palette size', () => {
    const size = PALETTES.extended.light.length;
    expect(getColorByIndex(size, 'light')).toBe(PALETTES.extended.light[0]);
    expect(getColorByIndex(size + 1, 'light')).toBe(PALETTES.extended.light[1]);
  });

  it('uses the dark palette when theme is dark', () => {
    expect(getColorByIndex(0, 'dark')).toBe(PALETTES.extended.dark[0]);
  });
});

// ---- getColorForKey --------------------------------------------------------

describe('getColorForKey', () => {
  it('returns a hex color string', () => {
    const color = getColorForKey('my-key', 'light');
    expect(color).toMatch(/^#[0-9a-fA-F]{6}$/);
  });

  it('is deterministic: same key always returns the same color', () => {
    const c1 = getColorForKey('alpha', 'light');
    const c2 = getColorForKey('alpha', 'light');
    expect(c1).toBe(c2);
  });

  it('assigns different colors to different keys (up to palette size)', () => {
    const palette = getActivePalette('light');
    const keys = Array.from({ length: palette.length }, (_, i) => `key-${i}`);
    const colors = keys.map(k => getColorForKey(k, 'light'));
    const unique = new Set(colors);
    expect(unique.size).toBe(palette.length);
  });

  it('caches the assignment so a key added later does not displace an earlier one', () => {
    const c1 = getColorForKey('first', 'light');
    getColorForKey('second', 'light');
    expect(getColorForKey('first', 'light')).toBe(c1);
  });

  it('uses the dark palette when theme is dark', () => {
    const color = getColorForKey('k', 'dark');
    expect(PALETTES.extended.dark).toContain(color);
  });
});

// ---- assignColors ----------------------------------------------------------

describe('assignColors', () => {
  it('assigns palette colors in order to the given keys', () => {
    const palette = getActivePalette('light');
    const result = assignColors(['a', 'b', 'c'], 'light');
    expect(result.a).toBe(palette[0]);
    expect(result.b).toBe(palette[1]);
    expect(result.c).toBe(palette[2]);
  });

  it('wraps around when there are more keys than palette colors', () => {
    const palette = getActivePalette('light');
    const size = palette.length;
    const keys = Array.from({ length: size + 2 }, (_, i) => `k${i}`);
    const result = assignColors(keys, 'light');
    expect(result[`k${size}`]).toBe(palette[0]);
    expect(result[`k${size + 1}`]).toBe(palette[1]);
  });

  it('returns an empty record for an empty keys array', () => {
    expect(assignColors([], 'light')).toEqual({});
  });
});

// ---- createCapacitiesColorFn -----------------------------------------------

describe('createCapacitiesColorFn', () => {
  it('uses ordered palette assignment for multiple capacities', () => {
    const palette = getActivePalette('light');
    const fn = createCapacitiesColorFn(['cpu', 'mem'], 'light');
    expect(fn('cpu')).toBe(palette[0]);
    expect(fn('mem')).toBe(palette[1]);
  });

  it('uses key-based coloring for a single capacity', () => {
    const fn = createCapacitiesColorFn(['cpu'], 'light');
    // getColorForKey is called at construction time; calling it again returns the cached assignment
    expect(fn('cpu')).toBe(getColorForKey('cpu', 'light'));
  });

  it('falls back to getColorForKey for unknown capacity names', () => {
    const fn = createCapacitiesColorFn(['cpu', 'mem'], 'light');
    const unknown = fn('disk');
    expect(unknown).toMatch(/^#[0-9a-fA-F]{6}$/);
  });

  it('returns a color string for an empty capacities array', () => {
    const fn = createCapacitiesColorFn([], 'light');
    expect(fn('anything')).toMatch(/^#[0-9a-fA-F]{6}$/);
  });
});

// ---- createFsmTypeColorFn --------------------------------------------------

describe('createFsmTypeColorFn', () => {
  const fsmTypes = {
    MyFsm: {
      name: 'MyFsm',
      states: [{ name: 'idle' }, { name: 'running' }, { name: 'done' }],
    },
  };

  it('assigns colors by state index for known states', () => {
    const fn = createFsmTypeColorFn(fsmTypes, 'light');
    expect(fn('idle')).toBe(getColorByIndex(0, 'light'));
    expect(fn('running')).toBe(getColorByIndex(1, 'light'));
    expect(fn('done')).toBe(getColorByIndex(2, 'light'));
  });

  it('falls back to getColorForKey for unknown state names', () => {
    const fn = createFsmTypeColorFn(fsmTypes, 'light');
    const color = fn('unknown-state');
    expect(color).toMatch(/^#[0-9a-fA-F]{6}$/);
  });

  it('works with an empty fsmTypes object', () => {
    const fn = createFsmTypeColorFn({}, 'light');
    expect(fn('any-state')).toMatch(/^#[0-9a-fA-F]{6}$/);
  });
});

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

// ---- darkenColor -----------------------------------------------------------

describe('darkenColor', () => {
  it('returns the original color unchanged at amount 0', () => {
    expect(darkenColor('#ffffff', 0)).toBe('#ffffff');
  });

  it('returns pure black at amount 1', () => {
    expect(darkenColor('#ffffff', 1)).toBe('#000000');
  });

  it('darkens a color by 50%', () => {
    // #ffffff → r=255, g=255, b=255; *0.5 → 128 = 0x80 each
    expect(darkenColor('#ffffff', 0.5)).toBe('#808080');
  });

  it('handles a non-trivial color', () => {
    // #0072B2 → r=0, g=114=0x72, b=178=0xB2; *0.5 → r=0, g=57=0x39, b=89=0x59
    expect(darkenColor('#0072B2', 0.5)).toBe('#003959');
  });

  it('clamps amount above 1 to pure black', () => {
    expect(darkenColor('#ffffff', 2)).toBe('#000000');
  });

  it('clamps amount below 0 to the original color', () => {
    expect(darkenColor('#aabbcc', -1)).toBe('#aabbcc');
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

// ---- getOperationTypeColor -------------------------------------------------

describe('getOperationTypeColor', () => {
  it('returns a hex color string', () => {
    expect(getOperationTypeColor('Scan')).toMatch(/^#[0-9a-fA-F]{6}$/);
  });

  it('is deterministic for the same operation type', () => {
    expect(getOperationTypeColor('Join')).toBe(getOperationTypeColor('Join'));
  });

  it('is case-insensitive', () => {
    expect(getOperationTypeColor('SCAN')).toBe(getOperationTypeColor('scan'));
    expect(getOperationTypeColor('Scan')).toBe(getOperationTypeColor('scan'));
  });

  it('different operation types may have different colors', () => {
    // Hash-based, so not guaranteed — but common types should differ
    const colors = new Set(['Scan', 'Join', 'Aggregate', 'Sort'].map(getOperationTypeColor));
    expect(colors.size).toBeGreaterThan(1);
  });
});

// ---- buildOperatorColorMap -------------------------------------------------

describe('buildOperatorColorMap', () => {
  it('returns an empty map for an empty input', () => {
    expect(buildOperatorColorMap([])).toEqual(new Map());
  });

  it('returns a map entry for each unique type', () => {
    const map = buildOperatorColorMap(['Scan', 'Join', 'Aggregate']);
    expect(map.size).toBe(3);
    expect(map.has('scan')).toBe(true);
    expect(map.has('join')).toBe(true);
    expect(map.has('aggregate')).toBe(true);
  });

  it('deduplicates case-insensitively', () => {
    const map = buildOperatorColorMap(['Scan', 'SCAN', 'scan']);
    expect(map.size).toBe(1);
  });

  it('assigns distinct colors to distinct types (up to palette size)', () => {
    const types = ['alpha', 'beta', 'gamma', 'delta'];
    const map = buildOperatorColorMap(types);
    const colors = Array.from(map.values());
    const unique = new Set(colors);
    expect(unique.size).toBe(types.length);
  });

  it('is deterministic: same input always produces the same map', () => {
    const m1 = buildOperatorColorMap(['Scan', 'Join']);
    const m2 = buildOperatorColorMap(['Scan', 'Join']);
    expect(m1.get('scan')).toBe(m2.get('scan'));
    expect(m1.get('join')).toBe(m2.get('join'));
  });
});

// ---- continuousColor -------------------------------------------------------

describe('continuousColor', () => {
  it('returns the neutral color at t=0 (light mode, blue)', () => {
    // blendToColor(59, 130, 246, 0, [229, 231, 235]) → #e5e7eb
    expect(continuousColor(0, 'blue')).toBe('#e5e7eb');
  });

  it('returns the full color at t=1 (light mode, blue)', () => {
    // blendToColor(59, 130, 246, 1, [229,231,235]) → #3b82f6
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
