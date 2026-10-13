// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

/** @type {import('dependency-cruiser').IConfiguration} */
export default {
  forbidden: [
    {
      name: 'no-circular-dependencies',
      severity: 'error',
      from: {},
      to: {
        circular: true,
      },
    },
    {
      name: 'packages-cannot-import-apps',
      severity: 'error',
      from: { path: '^packages/@quent/' },
      to: { path: '^(src|simulator)/' },
    },
    {
      name: 'utils-cannot-import-higher-packages',
      severity: 'error',
      from: { path: '^packages/@quent/utils/' },
      to: { path: '^packages/@quent/(client|hooks|components)/' },
    },
    {
      name: 'client-cannot-import-higher-packages',
      severity: 'error',
      from: { path: '^packages/@quent/client/' },
      to: { path: '^packages/@quent/(hooks|components)/' },
    },
    {
      name: 'hooks-cannot-import-components',
      severity: 'error',
      from: { path: '^packages/@quent/hooks/' },
      to: { path: '^packages/@quent/components/' },
    },
    {
      name: 'generic-ui-cannot-import-quent-runtime',
      severity: 'error',
      from: { path: '^packages/@quent/components/src/ui/' },
      to: {
        path: [
          '^(src|simulator)/',
          '^generated/ts-bindings/',
          '^packages/@quent/(client|hooks)/',
          '^packages/@quent/components/src/(?!ui/|lib/sliderField[.]utils[.]ts$)',
        ],
      },
    },
    {
      name: 'app-cannot-import-generated-bindings',
      severity: 'error',
      from: { path: '^(src|simulator)/' },
      to: { path: '^generated/ts-bindings/' },
    },
  ],
  options: {
    exclude: {
      path: '(^|/)node_modules(?:/|$)',
    },
    doNotFollow: {
      path: ['(^|/)node_modules(?:/|$)', '^generated/ts-bindings/'],
    },
    includeOnly: [
      '^src/',
      '^simulator/',
      '^packages/@quent/(utils|client|hooks|components)/src/',
      '^generated/ts-bindings/',
    ],
    moduleSystems: ['es6', 'cjs'],
    parser: 'tsc',
    tsPreCompilationDeps: true,
    enhancedResolveOptions: {
      exportsFields: ['exports'],
      conditionNames: ['import', 'types', 'default'],
      extensions: ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs', '.json'],
    },
    skipAnalysisNotInRules: true,
  },
};
