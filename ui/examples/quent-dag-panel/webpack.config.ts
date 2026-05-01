import type { Configuration } from 'webpack';
import { merge } from 'webpack-merge';
import path from 'path';
import grafanaConfig, { type Env } from './.config/webpack/webpack.config';

const config = async (env: Env): Promise<Configuration> => {
  const baseConfig = await grafanaConfig(env);

  return merge(baseConfig, {
    resolve: {
      alias: {
        // Force ELK bundled browser build to avoid unresolved "web-worker".
        elkjs: 'elkjs/lib/elk.bundled.js',
        // Ensure @quent/client hooks and panel code share one React Query instance.
        '@tanstack/react-query': path.resolve(__dirname, 'node_modules/@tanstack/react-query'),
      },
    },
  });
};

export default config;
