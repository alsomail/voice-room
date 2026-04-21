import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    server: {
      deps: {
        // @exodus/bytes 是纯 ESM 包，jsdom 的 html-encoding-sniffer 依赖它。
        // 使用正则匹配所有子路径（如 encoding-lite.js），避免 CJS require() 失败。
        inline: [/^@exodus\/bytes/],
      },
    },
  },
});
