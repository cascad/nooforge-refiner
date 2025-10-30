import { defineConfig } from 'vite';
import solid from 'vite-plugin-solid';

export default defineConfig({
  plugins: [solid()],
  
  // Для совместимости с Tauri
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  
  // Оптимизация production сборки
  build: {
    target: ['es2021', 'chrome100', 'safari13'],
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
