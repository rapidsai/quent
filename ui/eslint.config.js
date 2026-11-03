// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import js from '@eslint/js';
import globals from 'globals';
import reactHooks from 'eslint-plugin-react-hooks';
import reactRefresh from 'eslint-plugin-react-refresh';
import tseslint from 'typescript-eslint';
import prettier from 'eslint-plugin-prettier/recommended';

const packageAliasRestrictions = [
  {
    group: ['@/**'],
    message: 'The @/ alias belongs to the application layer.',
  },
  {
    group: ['~quent/types/**'],
    message: 'Consume generated bindings through @quent/utils.',
  },
];

export default tseslint.config(
  {
    // `examples/*` are self-contained consumer apps with their own ESLint
    // toolchains (e.g. `examples/quent-dag-panel` uses `@grafana/eslint-config`
    // and is linted via its own `npm run lint`). The Grafana plugin's
    // `.config/` directory is also vendor-scaffolded and must not be modified.
    // Each example's own CI lints itself; we don't want the root workspace
    // lint to second-guess them with a different rule set.
    ignores: [
      'dist',
      'src/routeTree.gen.ts',
      'examples/**',
      '**/dist/**',
      '**/node_modules/**',
      'generated/ts-bindings/**',
      '.e2e-data/**',
      'playwright-report/**',
      'test-results/**',
    ],
  },
  {
    extends: [js.configs.recommended, ...tseslint.configs.recommended],
    files: ['**/*.{ts,tsx}'],
    languageOptions: {
      ecmaVersion: 2020,
      globals: globals.browser,
    },
    plugins: {
      'react-hooks': reactHooks,
      'react-refresh': reactRefresh,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      'react-hooks/set-state-in-effect': 'warn',
      'react-hooks/refs': 'warn',
      'react-hooks/purity': 'off',
      'react-refresh/only-export-components': 'off',
      'react-hooks/incompatible-library': 'off',
      'no-console': ['error', { allow: ['warn', 'error'] }],
      '@typescript-eslint/no-unused-vars': [
        'error',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],
    },
  },
  {
    files: ['packages/@quent/**/*.{ts,tsx}'],
    rules: {
      'no-restricted-imports': ['error', { patterns: packageAliasRestrictions }],
    },
  },
  {
    files: ['packages/@quent/utils/**/*.{ts,tsx}'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            ...packageAliasRestrictions,
            {
              group: ['@quent/client', '@quent/client/**', '@quent/hooks', '@quent/hooks/**'],
              message: '@quent/utils is the bottom Quent package layer.',
            },
            {
              group: ['@quent/components', '@quent/components/**'],
              message: '@quent/utils cannot import the component layer.',
            },
          ],
        },
      ],
    },
  },
  {
    files: ['packages/@quent/client/**/*.{ts,tsx}'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            ...packageAliasRestrictions,
            {
              group: [
                '@quent/hooks',
                '@quent/hooks/**',
                '@quent/components',
                '@quent/components/**',
              ],
              message: '@quent/client cannot import higher package layers.',
            },
          ],
        },
      ],
    },
  },
  {
    files: ['packages/@quent/hooks/**/*.{ts,tsx}'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            ...packageAliasRestrictions,
            {
              group: ['@quent/components', '@quent/components/**'],
              message: '@quent/hooks cannot import the component layer.',
            },
          ],
        },
      ],
    },
  },
  {
    files: ['packages/@quent/components/src/ui/**/*.{ts,tsx}'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          paths: [
            // Keep query-engine wire contracts above generic primitives.
            {
              name: '@quent/utils/query-engine',
              message: 'Generic UI cannot import query-engine contracts.',
            },
            {
              name: '@quent/utils',
              allowImportNames: ['clamp', 'cn'],
              message: 'Generic UI may temporarily import only cn and clamp from @quent/utils.',
            },
            {
              name: '@quent/client',
              message: 'Generic UI cannot call Quent APIs.',
            },
            {
              name: '@quent/hooks',
              message: 'Generic UI cannot use Quent state.',
            },
            {
              name: '@quent/components',
              message: 'Generic UI cannot import the higher component facade.',
            },
          ],
          patterns: [
            ...packageAliasRestrictions,
            {
              group: ['@quent/client/**', '@quent/hooks/**', '@quent/components/**'],
              message: 'Generic UI cannot import Quent runtime or domain layers.',
            },
            {
              regex:
                '^\\.\\./(?!(?:lib/sliderField[.]utils|badge|button|colors|data-text|pointer-tooltip-portal|thin-scroll)$)',
              message: 'Generic UI may only import its own directory.',
            },
          ],
        },
      ],
    },
  },
  {
    // Keep reusable Quent views below query-engine integration adapters.
    files: ['packages/@quent/components/src/quent-views/**/*.{ts,tsx}'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          paths: [
            // Keep compatibility-barrel wire types out of reusable views.
            {
              name: '@quent/utils',
              allowImportNames: ['bigintToChartNumber', 'formatBytes', 'formatQuantity'],
              message: 'Quent views may import only explicitly view-safe utilities.',
            },
            {
              name: '@quent/client',
              message: 'Quent views cannot call query-engine APIs.',
            },
            {
              name: '@quent/hooks',
              message: 'Quent views cannot use query-engine state.',
            },
            {
              name: '@quent/components',
              message: 'Quent views must not import the mixed component facade.',
            },
            {
              name: '@quent/utils/query-engine',
              message: 'Quent views cannot import query-engine wire contracts.',
            },
          ],
          patterns: [
            ...packageAliasRestrictions,
            {
              group: ['@quent/client/**', '@quent/hooks/**'],
              message: 'Quent views cannot import query-engine integrations.',
            },
            {
              regex: '^(?:\\.\\./)+query-engine(?:/|$)',
              message: 'Quent views cannot import the query-engine source zone.',
            },
            {
              regex: '(^|/)generated/ts-bindings/',
              message: 'Quent views cannot import generated bindings.',
            },
          ],
        },
      ],
    },
  },
  {
    files: ['src/**/*.{ts,tsx}', 'simulator/**/*.{ts,tsx}'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            {
              group: ['~quent/types/**'],
              message: 'Consume generated bindings through @quent/utils.',
            },
            {
              group: ['@quent/*/src/**'],
              message: 'Applications must import from public package exports.',
            },
          ],
        },
      ],
    },
  },
  {
    ...prettier,
    rules: {
      ...prettier.rules,
      curly: ['error', 'all'],
    },
  }
);
