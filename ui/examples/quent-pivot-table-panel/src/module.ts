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
    .addTextInput({
      path: 'partitionLabel',
      name: 'Partition label',
      description: 'Header for the outer (partition) index dimension.',
      defaultValue: DEFAULT_OPTIONS.partitionLabel,
      category: ['Index labels'],
    })
    .addTextInput({
      path: 'itemTypeLabel',
      name: 'Item-type label',
      description: 'Header for the middle (item_type) index dimension.',
      defaultValue: DEFAULT_OPTIONS.itemTypeLabel,
      category: ['Index labels'],
    })
    .addTextInput({
      path: 'itemLabel',
      name: 'Item label',
      description: 'Header for the innermost (item) index dimension.',
      defaultValue: DEFAULT_OPTIONS.itemLabel,
      category: ['Index labels'],
    })
);
