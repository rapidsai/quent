import path from 'path';
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { TanStackRouterVite } from '@tanstack/router-vite-plugin';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    react(),
    TanStackRouterVite({
      routeFileIgnorePattern: '.test.|.spec.',
    }),
  ],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      // TODO: Using ts bindings from quent for now this will need to change
      // to get bindings from webserver when we go that direction
      '~quent/types': path.resolve(__dirname, '../crates/server/ts-bindings'),
    },
  },
});
