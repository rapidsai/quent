// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

/** @type {import('dependency-cruiser').IConfiguration} */
export default {
  forbidden: [
    {
      // Keep the module graph acyclic so dependencies always have a clear direction.
      name: 'no-circular-dependencies',
      severity: 'error',
      from: {},
      to: {
        circular: true,
      },
    },
    {
      // Keep reusable packages independent of the application and simulator shells.
      name: 'packages-cannot-import-apps',
      severity: 'error',
      from: { path: '^packages/@quent/' },
      to: { path: '^(src|simulator)/' },
    },
    {
      // Keep @quent/utils as the foundation with no dependency on higher Quent packages.
      name: 'utils-cannot-import-higher-packages',
      severity: 'error',
      from: { path: '^packages/@quent/utils/' },
      to: { path: '^packages/@quent/(client|hooks|components)/' },
    },
    {
      // Keep @quent/client independent of shared state and rendering packages.
      name: 'client-cannot-import-higher-packages',
      severity: 'error',
      from: { path: '^packages/@quent/client/' },
      to: { path: '^packages/@quent/(hooks|components)/' },
    },
    {
      // Keep @quent/hooks independent of React component implementations.
      name: 'hooks-cannot-import-components',
      severity: 'error',
      from: { path: '^packages/@quent/hooks/' },
      to: { path: '^packages/@quent/components/' },
    },
    {
      // Keep generic UI free of app code, bindings, Quent runtime state, and domain components.
      name: 'generic-ui-cannot-import-quent-runtime',
      severity: 'error',
      from: { path: '^packages/@quent/components/src/ui/' },
      to: {
        path: [
          '^(src|simulator)/',
          '^generated/ts-bindings/',
          '^packages/@quent/(client|hooks)/',
          '^packages/@quent/utils/src/query-engine[.]ts$',
          '^packages/@quent/components/src/(?!ui/|lib/sliderField[.]utils[.]ts$)',
        ],
      },
    },
    {
      // Keep reusable Quent views below query-engine integration adapters.
      name: 'quent-views-cannot-import-query-engine',
      severity: 'error',
      from: { path: '^packages/@quent/components/src/quent-views/' },
      to: {
        path: [
          '^generated/ts-bindings/',
          '^packages/@quent/(client|hooks)/',
          '^packages/@quent/utils/src/query-engine[.]ts$',
          '^packages/@quent/components/src/query-engine/',
        ],
      },
    },
    {
      // Require applications to consume generated backend types through @quent/utils.
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
    tsConfig: {
      fileName: 'tsconfig.json',
    },
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
