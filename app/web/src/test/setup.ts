import '@testing-library/jest-dom';

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

