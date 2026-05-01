// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

/**
 * Options surfaced in Grafana's panel editor. Data is supplied via Grafana's
 * datasource pipeline (panel `targets`), so the only setting here is the
 * theme override — everything else is the responsibility of the query.
 */
export interface QuentPivotTablePanelOptions {
  /** Override the host theme detection. `auto` follows Grafana's `theme.isDark`. */
  themeMode: 'auto' | 'light' | 'dark';
}

export const DEFAULT_OPTIONS: QuentPivotTablePanelOptions = {
  themeMode: 'auto',
};
