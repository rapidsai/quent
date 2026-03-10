/**
 * One operator with an active span, normalized for chart consumption.
 * Time is in milliseconds (aligned with timeline startTime).
 */
export type OperatorActiveSpanEntry = {
  operatorId: string;
  /** Display name (instance name or type name). */
  label: string;
  /** Operator type name (e.g. "Scan", "Join"). */
  typeName: string;
  startMs: number;
  endMs: number;
  /** Row index for categorical y-axis (0-based). */
  rowIndex: number;
};
