/**
 * RoleGuard — 角色守卫组件（T-20014 R1 修复）
 *
 * 职责：
 *   - 在 AuthGuard 之后使用，进一步校验当前用户角色
 *   - 角色在 allowedRoles 内：渲染子路由（<Outlet />）
 *   - 角色不在 allowedRoles 或 admin 为 null：重定向到 fallback（默认 /403）
 *
 * 使用方式（路由配置）：
 *   <Route element={<AuthGuard />}>
 *     <Route element={<RoleGuard allowedRoles={['super_admin', 'operator', 'cs']} />}>
 *       <Route path="/rooms/governance" element={<GovernanceLogsPage />} />
 *     </Route>
 *   </Route>
 */

import { Navigate, Outlet } from 'react-router-dom';
import { useAuthStore } from '../stores/useAuthStore';

export interface RoleGuardProps {
  /** 允许访问的角色列表 */
  allowedRoles: string[];
  /** 角色不匹配时的重定向目标（默认 /403） */
  fallback?: string;
}

export function RoleGuard({ allowedRoles, fallback = '/403' }: RoleGuardProps) {
  const admin = useAuthStore((s) => s.admin);

  if (!admin || !allowedRoles.includes(admin.role)) {
    return <Navigate to={fallback} replace />;
  }

  return <Outlet />;
}
