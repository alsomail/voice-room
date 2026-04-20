/**
 * App — 根组件
 *
 * T-20002：接入 react-router-dom BrowserRouter + AppRoutes 路由配置
 */
import { BrowserRouter } from 'react-router-dom';
import '../i18n';
import { AppRoutes } from '../router/index';

export function App() {
  return (
    <BrowserRouter>
      <AppRoutes />
    </BrowserRouter>
  );
}
