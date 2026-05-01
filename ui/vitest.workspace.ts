import { defineWorkspace } from 'vitest/config';

export default defineWorkspace([
  // App tests -- existing config unchanged
  './vitest.config.ts',
  // Per-package configs (picked up when created in later phases)
  './packages/@quent/*/vitest.config.ts',
]);
