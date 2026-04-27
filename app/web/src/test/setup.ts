import '@testing-library/jest-dom';
import { vi } from 'vitest';

/**
 * T-20020 测试环境默认 stub Web env 四字段，避免 webEnv 模块顶层 fail-fast
 * 导致全部用例集体红。需要测试 throw 路径的用例请在 beforeEach 内
 * `vi.unstubAllEnvs()` 后再 stubEnv。
 */
vi.stubEnv('VITE_API_BASE_URL', 'http://127.0.0.1:3000/api');
vi.stubEnv('VITE_WS_URL', 'ws://127.0.0.1:3000/ws');
vi.stubEnv('VITE_ADMIN_API_BASE_URL', 'http://127.0.0.1:3001/api/v1/admin');
vi.stubEnv(
  'VITE_ANALYTICS_ENDPOINT',
  'https://analytics-test.example.com/collect',
);

/**
 * jsdom 环境缺少 window.matchMedia，Ant Design 的响应式栅格（Row/Col）依赖它。
 * 在此补充最小 stub，避免测试中 TypeError。
 */
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string): MediaQueryList => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
});

/**
 * jsdom 环境缺少 ResizeObserver，Ant Design v6 的 rc-resize-observer 依赖它。
 * 提供最小 stub，避免测试中 ReferenceError。
 */
class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}

Object.defineProperty(window, 'ResizeObserver', {
  writable: true,
  value: ResizeObserverStub,
});

