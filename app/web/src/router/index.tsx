/**
 * 路由配置（T-20002）
 *
 * 路由结构：
 *   /             → 重定向到 /dashboard（已认证）或 /login（未认证）
 *   /login        → LoginPage（公开，已认证时重定向到 /dashboard）
 *   /dashboard    → DashboardPage（受 AuthGuard + AppLayout 保护）
 *   /rooms        → RoomsPage（受 AuthGuard + AppLayout 保护，T-20004）
 *   /users        → UsersPage（受 AuthGuard + AppLayout 保护，T-20006）
 *   /logs         → LogsPage（受 AuthGuard + AppLayout 保护，T-20009）
 *   /gifts        → GiftManagementPage（受 AuthGuard + AppLayout 保护，T-20012）
 *   /rooms/governance → GovernanceLogsPage（受 AuthGuard + AppLayout 保护，T-20014）
 */

import { Routes, Route, Navigate } from 'react-router-dom';
import { AuthGuard } from '../components/AuthGuard';
import { RoleGuard } from '../components/RoleGuard';
import { AppLayout } from '../app/AppLayout';
import { LoginPage } from '../pages/login/index';
import { DashboardPage } from '../pages/dashboard/index';
import { RoomsPage } from '../pages/rooms/index';
import { UsersPage } from '../pages/users/index';
import { LogsPage } from '../pages/logs/index';
import { GiftManagementPage } from '../features/gift/GiftManagementPage';
import { GovernanceLogsPage } from '../features/governance/GovernanceLogsPage';

export function AppRoutes() {
  return (
    <Routes>
      {/* 根路由：重定向到 /dashboard（AuthGuard 会二次检查认证状态） */}
      <Route path="/" element={<Navigate to="/dashboard" replace />} />

      {/* 公开路由 */}
      <Route path="/login" element={<LoginPage />} />

      {/* 受保护路由：AuthGuard 校验认证，AppLayout 提供侧栏布局 */}
      <Route element={<AuthGuard />}>
        <Route element={<AppLayout />}>
          <Route path="/dashboard" element={<DashboardPage />} />
          <Route path="/rooms" element={<RoomsPage />} />
          <Route path="/users" element={<UsersPage />} />
          <Route path="/logs" element={<LogsPage />} />
          {/* T-20012: 礼物管理页（RoleGuard 限制 super_admin / operator，T-20012 §RBAC） */}
          <Route
            element={
              <RoleGuard allowedRoles={['super_admin', 'operator']} />
            }
          >
            <Route path="/gifts" element={<GiftManagementPage />} />
          </Route>
          {/* T-20014: 治理日志页（RoleGuard 限制 super_admin / operator / cs） */}
          <Route
            element={
              <RoleGuard allowedRoles={['super_admin', 'operator', 'cs']} />
            }
          >
            <Route path="/rooms/governance" element={<GovernanceLogsPage />} />
          </Route>
        </Route>
      </Route>
    </Routes>
  );
}


