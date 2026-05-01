// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

/**
 * Options surfaced in Grafana's panel editor. Data is supplied via Grafana's
 * datasource pipeline (panel `targets`); the rest of these knobs let one
 * panel be reskinned for different datasets without code changes.
 */
export interface QuentPivotTablePanelOptions {
  /** Override the host theme detection. `auto` follows Grafana's `theme.isDark`. */
  themeMode: 'auto' | 'light' | 'dark';
  /** Header for the outer (`partition`) index dimension. */
  partitionLabel: string;
  /** Header for the middle (`item_type`) index dimension. */
  itemTypeLabel: string;
  /** Header for the innermost (`item`) index dimension. */
  itemLabel: string;
}

export const DEFAULT_OPTIONS: QuentPivotTablePanelOptions = {
  themeMode: 'auto',
  partitionLabel: 'Worker / Plan',
  itemTypeLabel: 'Operator Type',
  itemLabel: 'Operator',
};
