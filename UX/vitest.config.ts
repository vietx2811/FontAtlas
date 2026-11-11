import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: { alias: { '@': path.resolve(__dirname, '.') } },
  test: { environment: 'jsdom', setupFiles: ['./test/setup.ts'], include: ['features/**/__tests__/**/*.test.{ts,tsx}', 'components/**/__tests__/**/*.test.{ts,tsx}', 'api/**/__tests__/**/*.test.{ts,tsx}', 'bindings/__tests__/**/*.test.ts', 'store/**/__tests__/**/*.test.{ts,tsx}', 'i18n/__tests__/**/*.test.{ts,tsx}'] },
});
