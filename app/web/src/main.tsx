import React from 'react';
import ReactDOM from 'react-dom/client';

// T-20020: 启动期 fail-fast 校验。任一 VITE_* 缺失/空白将抛
// `[CONFIG ERROR] VITE_XXX must be set` 并阻塞后续 React 渲染。
import './core/config/env';
import { App } from './app/App';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
