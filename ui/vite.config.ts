import path from 'path';
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { TanStackRouterVite } from '@tanstack/router-vite-plugin';
import { visualizer } from 'rollup-plugin-visualizer';

const API_TARGET = process.env.VITE_API_TARGET || 'http://localhost:8000';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    react(),
    TanStackRouterVite({
      routeFileIgnorePattern: '.test.|.spec.',
    }),
    // Bundle analyzer - generates stats.html after build
    visualizer({
      filename: 'stats.html',
      open: false,
      gzipSize: true,
      brotliSize: true,
    }),
  ],
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          // Split large dependencies into separate chunks for better caching
          'react-vendor': ['react', 'react-dom'],
          tanstack: ['@tanstack/react-query', '@tanstack/react-router'],
          xyflow: ['@xyflow/react'],
          // echarts uses tree-shaking via @/lib/echarts.ts custom build
          echarts: ['echarts/core', 'echarts/charts', 'echarts/components', 'echarts/renderers'],
          // elkjs is handled separately via alias to bundled version
        },
      },
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      // TODO: Using ts bindings from quent for now this will need to change
      // to get bindings from webserver when we go that direction
      '~quent/types': path.resolve(__dirname, '../crates/server/ts-bindings'),
      // Force elkjs to use bundled version (avoids web-worker module resolution issues)
      elkjs: 'elkjs/lib/elk.bundled.js',
    },
  },
  server: {
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
