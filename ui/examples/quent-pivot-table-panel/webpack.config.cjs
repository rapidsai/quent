// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

/* eslint-env node */
/**
 * Minimal webpack config for a Grafana panel plugin.
 *
 * In a real Grafana plugin you would scaffold with `pnpm dlx
 * @grafana/create-plugin` and *extend* its `.config/webpack/webpack.config.ts`
 * via `webpack-merge`. This example keeps things visible in a single file so
 * the integration with `@quent/*` workspace packages is obvious; the rules
 * below mirror what the scaffolded config would do (TS via SWC, externalized
 * `@grafana/*` and React, plugin.json + img/ copied to `dist/`).
 *
 * Deliberately written as `.cjs` (not `.ts`) — webpack-cli's TS config
 * loader pulls in `rechoir`, which in pnpm's strict node_modules layout
 * fails to resolve `resolve`. CJS sidesteps the whole interpreter dance.
 */
const path = require('node:path');
const CopyWebpackPlugin = require('copy-webpack-plugin');

const PLUGIN_ID = 'quent-pivottable-panel';

/** @type {(env: unknown, argv?: { mode?: 'development' | 'production' }) => import('webpack').Configuration} */
module.exports = (_env, argv = {}) => {
  const mode = argv.mode ?? 'production';
  const isProd = mode === 'production';

  return {
    mode,
    target: 'web',
    context: path.resolve(__dirname, 'src'),
    entry: { module: './module.ts' },
    output: {
      clean: true,
      filename: '[name].js',
      path: path.resolve(__dirname, 'dist'),
      library: { type: 'amd' },
      publicPath: `public/plugins/${PLUGIN_ID}/`,
      uniqueName: PLUGIN_ID,
    },
    devtool: isProd ? false : 'source-map',
    resolve: {
      extensions: ['.ts', '.tsx', '.js', '.jsx', '.mjs'],
      // Workspace packages ship TS source; webpack must follow symlinks
      // into `node_modules/@quent/*` to find it.
      symlinks: true,
      alias: {
        // Force the bundled ELK build (avoids web-worker module resolution).
        // `@quent/components`'s DAGChart pulls this in; pivot table doesn't,
        // but keep it here so the example stays useful as a copy-paste base.
        elkjs: 'elkjs/lib/elk.bundled.js',
      },
    },
    module: {
      rules: [
        {
          test: /\.[jt]sx?$/,
          exclude: /node_modules\/(?!@quent\/)/,
          use: {
            loader: 'swc-loader',
            options: {
              jsc: {
                parser: { syntax: 'typescript', tsx: true, decorators: false },
                target: 'es2022',
                transform: {
                  react: {
                    runtime: 'automatic',
                    // ALWAYS use the *production* JSX runtime
                    // (`react/jsx-runtime`), even when webpack is in
                    // development mode. Grafana ships React as a production
                    // build, so the dev runtime's reads of
                    // `ReactSharedInternals.recentlyCreatedOwnerStacks`
                    // (and friends) hit `undefined` and crash. The prod
                    // runtime only calls `React.createElement`-style
                    // exports that exist on every React build.
                    development: false,
                    refresh: false,
                  },
                },
              },
            },
          },
        },
        {
          test: /\.css$/i,
          use: [
            'style-loader',
            { loader: 'css-loader', options: { url: false, importLoaders: 1 } },
            'postcss-loader',
          ],
        },
      ],
    },
    // Grafana's plugin loader provides these at runtime; do not bundle them.
    //
    // Only the bare specifiers are externalized — Grafana's SystemJS does
    // *not* register submodule paths like `react/jsx-runtime` or
    // `react-dom/client`, so externalizing those triggers a runtime
    // "SystemJS: failed to resolve" error. The JSX runtime itself is
    // small; we let webpack bundle it, and inside the bundle it `import`s
    // `'react'`, which goes through the bare external and resolves to
    // Grafana's React instance — so we still end up with exactly one
    // React in the page.
    externalsType: 'amd',
    externals: [
      'react',
      'react-dom',
      '@grafana/data',
      '@grafana/runtime',
      '@grafana/ui',
      'lodash',
      'moment',
      'rxjs',
    ],
    plugins: [
      new CopyWebpackPlugin({
        patterns: [
          { from: 'plugin.json', to: '.' },
          { from: 'img/**/*', to: '.', noErrorOnMissing: true },
          { from: '../README.md', to: '.', noErrorOnMissing: true },
        ],
      }),
    ],
  };
};
