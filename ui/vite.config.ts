import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

const AGENT = 'http://127.0.0.1:47800';
const EDITOR = 'http://127.0.0.1:47801';

export default defineConfig({
  plugins: [svelte()],
  build: {
    // Old phones: Android 8 ships with an old WebView/Chrome if it was never updated.
    // iPhones: Safari 13 and newer.
    target: ['chrome75', 'safari13'],
    cssTarget: ['chrome75', 'safari13'],
    rollupOptions: {
      input: {
        // The phone's dashboard and the editor that runs on the PC.
        main: resolve(import.meta.dirname, 'index.html'),
        editor: resolve(import.meta.dirname, 'editor.html'),
      },
    },
  },
  server: {
    proxy: {
      '/ws': { target: AGENT, ws: true },
      '/art': AGENT,
      '/user': AGENT,
      '/api': EDITOR,
    },
  },
});
