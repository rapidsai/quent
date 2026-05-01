// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

/**
 * Options surfaced in Grafana's panel editor. The plugin is fully driven by
 * these — there is no Grafana datasource adapter; the panel calls the Quent
 * API directly via `@quent/client`.
 */
export interface QuentPivotTablePanelOptions {
  /** Quent API base URL, e.g. `https://quent.example.com/api`. */
  apiBaseUrl: string;
  /** Engine id (UUID/string) the query lives under. */
  engineId: string;
  /** Query id to fetch the QueryBundle for. */
  queryId: string;
  /** Override the host theme detection. `auto` follows Grafana's `theme.isDark`. */
  themeMode: 'auto' | 'light' | 'dark';
}

export const DEFAULT_OPTIONS: QuentPivotTablePanelOptions = {
  apiBaseUrl: '/api',
  engineId: '',
  queryId: '',
  themeMode: 'auto',
};
