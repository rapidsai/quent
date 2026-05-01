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
const fs = require('node:fs');
const CopyWebpackPlugin = require('copy-webpack-plugin');

const PLUGIN_ID = 'quent-pivottable-panel';

/**
 * React 18 names the prod jsx-runtime `react-jsx-runtime.production.min.js`;
 * React 19 dropped the `.min` suffix. Probe both so the alias works against
 * whichever React the host expects (Grafana 12 currently ships React 18).
 */
function resolveProdJsxRuntime() {
  const reactDir = path.dirname(require.resolve('react/package.json'));
  const candidates = [
    'cjs/react-jsx-runtime.production.min.js',
    'cjs/react-jsx-runtime.production.js',
  ];
  for (const rel of candidates) {
    const abs = path.resolve(reactDir, rel);
    if (fs.existsSync(abs)) return abs;
  }
  throw new Error(
    `Could not locate a production react/jsx-runtime in ${reactDir}. ` +
      `Tried: ${candidates.join(', ')}`
  );
}

const PROD_JSX_RUNTIME = resolveProdJsxRuntime();

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
        // ALWAYS bundle the *production* JSX runtime, regardless of webpack
        // mode. The swc-loader option `transform.react.development = false`
        // controls which specifier swc emits (`react/jsx-runtime` vs
        // `react/jsx-dev-runtime`), but the resolution of `react/jsx-runtime`
        // through React's package.json `exports` field still picks
        // `react-jsx-runtime.development.js` when webpack runs in
        // `mode: 'development'`. The dev runtime reads
        // `ReactSharedInternals.recentlyCreatedOwnerStacks`, which only
        // exists on a development React build — Grafana ships a production
        // React, so the property is `undefined` and the panel crashes with
        // "Cannot read properties of undefined (reading
        // 'recentlyCreatedOwnerStacks')". Pinning the alias here makes
        // `pnpm dev` and `pnpm build` produce equivalent runtime behavior.
        // React's `exports` blocks `./cjs/*` from `require.resolve`, so we
        // build the path manually relative to the plain `react` entry.
        // (PROD_JSX_RUNTIME is computed above, probing for both React 18
        // and React 19 filename conventions.)
        'react/jsx-runtime': PROD_JSX_RUNTIME,
        'react/jsx-dev-runtime': PROD_JSX_RUNTIME,
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
