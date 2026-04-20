/**
 * AuthGuard — 路由守卫组件（T-20002）
 *
 * 职责：
 *   - 每次渲染时调用 checkAuth()，重新验证 token 有效性
 *   - 已认证且 token 未过期：渲染子路由（<Outlet />）
 *   - 未认证或 token 已过期：重定向到 /login
 *
 * [HIGH-H02] 修复说明：
 *   原实现读取 isAuthenticated（仅模块初始化时计算），用户登录后 token 在使用中过期
 *   但守卫无感知，仍允许访问受保护页面。
 *   改为调用 checkAuth()，每次路由进入时重新检测 JWT exp 字段。
 */

import { Navigate, Outlet } from 'react-router-dom';
import { useAuthStore } from '../stores/useAuthStore';

export function AuthGuard() {
  const checkAuth = useAuthStore((s) => s.checkAuth);

  if (!checkAuth()) {
    return <Navigate to="/login" replace />;
  }

  return <Outlet />;
}
