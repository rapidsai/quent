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
 */
import path from 'node:path';
import type { Configuration } from 'webpack';
import CopyWebpackPlugin from 'copy-webpack-plugin';

const PLUGIN_ID = 'quent-pivottable-panel';

const config = (_env: unknown, argv: { mode?: 'development' | 'production' } = {}): Configuration => {
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
                transform: { react: { runtime: 'automatic' } },
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

export default config;
