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
        // @exodus/bytes 是纯 ESM 包，jsdom 的 html-encoding-sniffer 依赖它
        // 通过 inline 让 Vite transform 将其转为可在 CJS 环境中使用的格式
        inline: ['@exodus/bytes'],
      },
    },
  },
});
