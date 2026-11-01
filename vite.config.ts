import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Порт 1431, чтобы не пересекаться с YummyAnime Desktop (1430).
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  // Не следим за src-tauri: туда пишет компилятор Rust, и Vite ругается на занятые файлы.
  server: { port: 1431, strictPort: true, watch: { ignored: ['**/src-tauri/**'] } },
  build: { target: 'es2022' },
  test: { include: ['src/**/*.test.ts'] },
});
