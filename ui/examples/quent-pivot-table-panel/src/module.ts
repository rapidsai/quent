// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import { PanelPlugin } from '@grafana/data';
import { QuentPivotTablePanel } from './QuentPivotTablePanel';
import { DEFAULT_OPTIONS, type QuentPivotTablePanelOptions } from './types';
import './styles.css';

export const plugin = new PanelPlugin<QuentPivotTablePanelOptions>(
  QuentPivotTablePanel
).setPanelOptions(builder =>
  builder
    .addTextInput({
      path: 'apiBaseUrl',
      name: 'API base URL',
      description: 'Quent server API root, e.g. https://quent.example.com/api',
      defaultValue: DEFAULT_OPTIONS.apiBaseUrl,
    })
    .addTextInput({
      path: 'engineId',
      name: 'Engine ID',
      defaultValue: DEFAULT_OPTIONS.engineId,
    })
    .addTextInput({
      path: 'queryId',
      name: 'Query ID',
      defaultValue: DEFAULT_OPTIONS.queryId,
    })
    .addRadio({
      path: 'themeMode',
      name: 'Theme mode',
      description: '`auto` follows the Grafana theme; light/dark force a mode.',
      defaultValue: DEFAULT_OPTIONS.themeMode,
      settings: {
        options: [
          { value: 'auto', label: 'Auto' },
          { value: 'light', label: 'Light' },
          { value: 'dark', label: 'Dark' },
        ],
      },
    })
);
