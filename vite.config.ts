import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath, URL } from 'node:url';
import { readFileSync } from 'node:fs';

const pkg = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf-8'));

export default defineConfig({
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
  plugins: [svelte()],
  resolve: {
    alias: {
      $lib: fileURLToPath(new URL('./core/src/lib', import.meta.url)),
    },
  },
  server: {
    open: false,
  },
});
