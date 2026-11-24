// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

export type RowWithGroupKeys = { groupKeys: Array<{ id: string }> };

export function computeRowSpans<T extends RowWithGroupKeys>(rows: T[]): (number | null)[][] {
  const numCols = rows[0]?.groupKeys.length ?? 0;
  const spans: (number | null)[][] = rows.map(() => new Array(numCols).fill(null));
  if (rows.length === 0) {
    return spans;
  }

  for (let col = 0; col < numCols; col++) {
    let start = 0;
    for (let i = 1; i <= rows.length; i++) {
      const changed =
        i === rows.length ||
        rows[i].groupKeys.slice(0, col + 1).some((gk, j) => gk.id !== rows[i - 1].groupKeys[j]?.id);
      const parentChanged =
        col > 0 &&
        i < rows.length &&
        rows[i].groupKeys.slice(0, col).some((gk, j) => gk.id !== rows[start].groupKeys[j]?.id);
      if (changed || parentChanged) {
        spans[start][col] = i - start;
        start = i;
      }
    }
  }
  return spans;
}
