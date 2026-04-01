// Utilities
export { cn } from './cn';
export { parseJsonWithBigInt } from './parseJsonWithBigInt';

// Color utilities
export {
  PALETTES,
  getColorForKey,
  assignColors,
  getColorByIndex,
  getOperationTypeColor,
  withOpacity,
  resetColorAssignments,
  lightenColor,
  darkenColor,
  getActivePalette,
  setActivePalette,
  getPalette,
  createStripePattern,
  createDotPattern,
  createCrosshatchPattern,
  BLACK,
  WHITE,
} from './colors';
export type { PaletteName, ChartColor } from './colors';

// Formatter utilities
export { formatDuration, formatDurationForWindow, formatQuantity } from './formatters';

// Rust-generated TypeScript types
export * from './types/index';
