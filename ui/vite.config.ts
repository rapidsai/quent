// vp migrate: merged .oxlintrc.json (66a68de5ae0f8a6ba38c77d2e865421e).
// SPDX-FileCopyrightText: Copyright (c) 2026, NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

import path from 'path';
import { defineConfig } from 'vite-plus';
import react from '@vitejs/plugin-react';
import { TanStackRouterVite } from '@tanstack/router-vite-plugin';
import { visualizer } from 'rollup-plugin-visualizer';
import tailwindcss from '@tailwindcss/vite';

const API_TARGET = process.env.VITE_API_TARGET || 'http://localhost:8080';

/** Ensures JS chunks get high fetch priority so they load before competing API requests. */
function vitePluginScriptPriority() {
  return {
    name: 'vite-plugin-script-priority',
    transformIndexHtml(html: string) {
      return html
        .replace(/<script(\s[^>]*?)(\s*\/?)>/gi, (_, attrs, close) =>
          attrs.includes('fetchpriority')
            ? `<script${attrs}${close}>`
            : `<script fetchpriority="high"${attrs}${close}>`
        )
        .replace(/<link(\s+)([^>]*?rel=["']modulepreload["'][^>]*?)>/gi, (_, space, rest) =>
          rest.includes('fetchpriority')
            ? `<link${space}${rest}>`
            : `<link${space}fetchpriority="high" ${rest}>`
        );
    },
  };
}

function vendorChunk(id: string) {
  const normalizedId = id.replaceAll('\\', '/');
  const includesPackage = (packageName: string) =>
    normalizedId.includes(`/node_modules/${packageName}/`);

  if (includesPackage('react') || includesPackage('react-dom') || includesPackage('scheduler')) {
    return 'react-vendor';
  }
  if (includesPackage('@tanstack')) {
    return 'tanstack';
  }
  if (includesPackage('@radix-ui')) {
    return 'ui-vendor';
  }
  if (includesPackage('@xyflow')) {
    return 'xyflow';
  }
  if (includesPackage('zrender')) {
    return 'echarts-zrender';
  }
  if (includesPackage('echarts')) {
    return 'echarts-core';
  }
}

// https://vitejs.dev/config/
export default defineConfig({
  lint: {
    plugins: ['oxc', 'typescript', 'unicorn', 'import'],
    jsPlugins: [
      {
        name: 'vite-plus',
        specifier: 'vite-plus/oxlint-plugin',
      },
    ],
    categories: {
      correctness: 'warn',
    },
    env: {
      builtin: true,
    },
    ignorePatterns: [
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
    rules: {
      curly: ['error', 'all'],
      'import/no-cycle': ['error', { ignoreTypes: false }],
      'vite-plus/prefer-vite-plus-imports': 'error',
    },
    overrides: [
      {
        files: ['**/*.{ts,tsx}'],
        rules: {
          'constructor-super': 'off',
          'for-direction': 'error',
          'getter-return': 'off',
          'no-async-promise-executor': 'error',
          'no-case-declarations': 'error',
          'no-class-assign': 'off',
          'no-compare-neg-zero': 'error',
          'no-cond-assign': 'error',
          'no-const-assign': 'off',
          'no-constant-binary-expression': 'error',
          'no-constant-condition': 'error',
          'no-control-regex': 'error',
          'no-debugger': 'error',
          'no-delete-var': 'error',
          'no-dupe-class-members': 'off',
          'no-dupe-else-if': 'error',
          'no-dupe-keys': 'off',
          'no-duplicate-case': 'error',
          'no-empty': 'error',
          'no-empty-character-class': 'error',
          'no-empty-pattern': 'error',
          'no-empty-static-block': 'error',
          'no-ex-assign': 'error',
          'no-extra-boolean-cast': 'error',
          'no-fallthrough': 'error',
          'no-func-assign': 'off',
          'no-global-assign': 'error',
          'no-import-assign': 'off',
          'no-invalid-regexp': 'error',
          'no-irregular-whitespace': 'error',
          'no-loss-of-precision': 'error',
          'no-misleading-character-class': 'error',
          'no-new-native-nonconstructor': 'off',
          'no-nonoctal-decimal-escape': 'error',
          'no-obj-calls': 'off',
          'no-prototype-builtins': 'error',
          'no-redeclare': 'off',
          'no-regex-spaces': 'error',
          'no-self-assign': 'error',
          'no-setter-return': 'off',
          'no-shadow-restricted-names': 'error',
          'no-sparse-arrays': 'error',
          'no-this-before-super': 'off',
          'no-unassigned-vars': 'error',
          'no-undef': 'off',
          'no-unexpected-multiline': 'error',
          'no-unreachable': 'off',
          'no-unsafe-finally': 'error',
          'no-unsafe-negation': 'off',
          'no-unsafe-optional-chaining': 'error',
          'no-unused-labels': 'error',
          'no-unused-private-class-members': 'error',
          'no-unused-vars': [
            'error',
            {
              argsIgnorePattern: '^_',
              varsIgnorePattern: '^_',
            },
          ],
          'no-useless-assignment': 'error',
          'no-useless-backreference': 'error',
          'no-useless-catch': 'error',
          'no-useless-escape': 'error',
          'no-with': 'off',
          'preserve-caught-error': 'error',
          'require-yield': 'error',
          'use-isnan': 'error',
          'valid-typeof': 'error',
          'no-var': 'error',
          'prefer-const': 'error',
          'prefer-rest-params': 'error',
          'prefer-spread': 'error',
          'no-array-constructor': 'error',
          'no-unused-expressions': 'error',
          'typescript/ban-ts-comment': 'error',
          'typescript/no-duplicate-enum-values': 'error',
          'typescript/no-empty-object-type': 'error',
          'typescript/no-explicit-any': 'error',
          'typescript/no-extra-non-null-assertion': 'error',
          'typescript/no-misused-new': 'error',
          'typescript/no-namespace': 'error',
          'typescript/no-non-null-asserted-optional-chain': 'error',
          'typescript/no-require-imports': 'error',
          'typescript/no-this-alias': 'error',
          'typescript/no-unnecessary-type-constraint': 'error',
          'typescript/no-unsafe-declaration-merging': 'error',
          'typescript/no-unsafe-function-type': 'error',
          'typescript/no-wrapper-object-types': 'error',
          'typescript/prefer-as-const': 'error',
          'typescript/prefer-namespace-keyword': 'error',
          'typescript/triple-slash-reference': 'error',
          'no-console': [
            'error',
            {
              allow: ['warn', 'error'],
            },
          ],
          'react/rules-of-hooks': 'error',
          'react/exhaustive-deps': 'warn',
          'react/static-components': 'error',
          'react/use-memo': 'error',
          'react/preserve-manual-memoization': 'error',
          'react/incompatible-library': 'off',
          'react/immutability': 'error',
          'react/globals': 'error',
          'react/refs': 'warn',
          'react/set-state-in-effect': 'warn',
          'react/error-boundaries': 'error',
          'react/purity': 'off',
          'react/set-state-in-render': 'error',
          'react/unsupported-syntax': 'warn',
          'react/only-export-components': 'off',
        },
        plugins: ['react'],
        env: {
          es2020: true,
          browser: true,
        },
      },
      {
        files: ['packages/@quent/**/*.{ts,tsx}'],
        rules: {
          'no-restricted-imports': [
            'error',
            {
              patterns: [
                {
                  group: ['@/**'],
                  message: 'The @/ alias belongs to the application layer.',
                },
                {
                  group: ['~quent/types/**'],
                  message: 'Consume generated bindings through @quent/utils.',
                },
                {
                  regex: '^(?:\\.\\./)+(?:src|simulator|generated/ts-bindings)(?:/|$)',
                  message:
                    'Reusable packages cannot import application or generated-binding internals.',
                },
              ],
            },
          ],
        },
      },
      {
        files: ['packages/@quent/utils/**/*.{ts,tsx}'],
        rules: {
          'no-restricted-imports': [
            'error',
            {
              patterns: [
                {
                  group: ['@/**'],
                  message: 'The @/ alias belongs to the application layer.',
                },
                {
                  group: ['~quent/types/**'],
                  message: 'Consume generated bindings through @quent/utils.',
                },
                {
                  group: ['@quent/client', '@quent/client/**', '@quent/hooks', '@quent/hooks/**'],
                  message: '@quent/utils is the bottom Quent package layer.',
                },
                {
                  group: ['@quent/components', '@quent/components/**'],
                  message: '@quent/utils cannot import the component layer.',
                },
                {
                  regex: '^(?:\\.\\./)+(?:client|hooks|components)(?:/|$)',
                  message: '@quent/utils cannot import higher package internals.',
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
                {
                  group: ['@/**'],
                  message: 'The @/ alias belongs to the application layer.',
                },
                {
                  group: ['~quent/types/**'],
                  message: 'Consume generated bindings through @quent/utils.',
                },
                {
                  group: [
                    '@quent/hooks',
                    '@quent/hooks/**',
                    '@quent/components',
                    '@quent/components/**',
                  ],
                  message: '@quent/client cannot import higher package layers.',
                },
                {
                  regex: '^(?:\\.\\./)+(?:hooks|components)(?:/|$)',
                  message: '@quent/client cannot import higher package internals.',
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
                {
                  group: ['@/**'],
                  message: 'The @/ alias belongs to the application layer.',
                },
                {
                  group: ['~quent/types/**'],
                  message: 'Consume generated bindings through @quent/utils.',
                },
                {
                  group: ['@quent/components', '@quent/components/**'],
                  message: '@quent/hooks cannot import the component layer.',
                },
                {
                  regex: '^(?:\\.\\./)+components(?:/|$)',
                  message: '@quent/hooks cannot import component internals.',
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
                {
                  group: ['@/**'],
                  message: 'The @/ alias belongs to the application layer.',
                },
                {
                  group: ['~quent/types/**'],
                  message: 'Consume generated bindings through @quent/utils.',
                },
                {
                  group: ['@quent/client/**', '@quent/hooks/**', '@quent/components/**'],
                  message: 'Generic UI cannot import Quent runtime or domain layers.',
                },
                {
                  group: ['../*', '!../lib/sliderField.utils'],
                  message: 'Generic UI may only import its own directory.',
                },
              ],
            },
          ],
        },
      },
      {
        files: ['packages/@quent/components/src/quent-views/**/*.{ts,tsx}'],
        rules: {
          'no-restricted-imports': [
            'error',
            {
              paths: [
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
                {
                  group: ['@/**'],
                  message: 'The @/ alias belongs to the application layer.',
                },
                {
                  group: ['~quent/types/**'],
                  message: 'Consume generated bindings through @quent/utils.',
                },
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
                {
                  regex: '^(?:\\.\\./)+generated/ts-bindings(?:/|$)',
                  message: 'Consume generated bindings through @quent/utils.',
                },
              ],
            },
          ],
        },
      },
    ],
    options: {
      typeAware: true,
      typeCheck: process.env.VP_LINT_ONLY !== '1',
    },
  },
  fmt: {
    semi: true,
    trailingComma: 'es5',
    singleQuote: true,
    printWidth: 100,
    tabWidth: 2,
    useTabs: false,
    arrowParens: 'avoid',
    endOfLine: 'lf',
    sortPackageJson: false,
    ignorePatterns: ['**/routeTree.gen.ts', 'dist', 'node_modules'],
  },
  plugins: [
    react(),
    vitePluginScriptPriority(),
    TanStackRouterVite({
      routesDirectory: path.resolve(__dirname, 'src/routes'),
      generatedRouteTree: path.resolve(__dirname, 'src/routeTree.gen.ts'),
      routeFileIgnorePattern: '.test.|.spec.',
    }),
    tailwindcss(),
    // Bundle analyzer - generates stats.html after build
    visualizer({
      filename: 'stats.html',
      open: false,
      gzipSize: true,
      brotliSize: true,
    }),
  ],
  build: {
    chunkSizeWarningLimit: 1337,
    rollupOptions: {
      output: {
        manualChunks: vendorChunk,
      },
    },
  },
  resolve: {
    dedupe: ['react', 'react-dom', 'jotai', '@tanstack/react-query', '@tanstack/react-router'],
    alias: {
      '@': path.resolve(__dirname, './src'),
      // TODO: Using ts bindings from quent for now this will need to change
      // to get bindings from webserver when we go that direction
      '~quent/types': path.resolve(__dirname, 'generated/ts-bindings'),
      // Force elkjs to use bundled version (avoids web-worker module resolution issues)
      elkjs: 'elkjs/lib/elk.bundled.js',
    },
  },
  optimizeDeps: {
    // Workspace packages must NOT be pre-bundled. Pre-bundling collapses each
    // package's source into a single optimized chunk in `node_modules/.vite/`,
    // which means saving any file under `packages/@quent/*/src/` triggers a
    // full page reload ("new dependencies optimized") instead of surgical HMR.
    // Excluding them keeps their source in Vite's on-demand transform pipeline
    // alongside `src/`, so React Fast Refresh works across package boundaries.
    exclude: ['@quent/components', '@quent/hooks', '@quent/client', '@quent/utils'],
    include: [
      // echarts-for-react is a CJS peer dep of @quent/components; must be pre-bundled
      // here so Vite converts it to ESM with a proper default export rather than
      // serving the raw module.exports object to the browser.
      'echarts-for-react',
    ],
  },
  server: {
    watch: {
      followSymlinks: true,
    },
    proxy: {
      '/api': {
        target: API_TARGET,
        changeOrigin: true,
        secure: false,
        followRedirects: true,
        configure: proxy => {
          proxy.on('proxyRes', proxyRes => {
            // Remove CORS headers from backend since proxy handles it
            delete proxyRes.headers['access-control-allow-origin'];
            delete proxyRes.headers['access-control-allow-credentials'];
          });
        },
      },
    },
  },
  preview: {
    proxy: {
      '/api': {
        target: API_TARGET,
        changeOrigin: true,
        secure: false,
        followRedirects: true,
        configure: proxy => {
          proxy.on('proxyRes', proxyRes => {
            delete proxyRes.headers['access-control-allow-origin'];
            delete proxyRes.headers['access-control-allow-credentials'];
          });
        },
      },
    },
  },
});
