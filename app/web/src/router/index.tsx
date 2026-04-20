/**
 * 路由配置（T-20002）
 *
 * 路由结构：
 *   /             → 重定向到 /dashboard（已认证）或 /login（未认证）
 *   /login        → LoginPage（公开，已认证时重定向到 /dashboard）
 *   /dashboard    → DashboardPage（受 AuthGuard 保护）
 *   /rooms        → RoomsPage（受 AuthGuard 保护，T-20004）
 *   /users        → UsersPage（受 AuthGuard 保护，T-20006）
 *   /logs         → LogsPage（受 AuthGuard 保护，T-20009）
 */

import { Routes, Route, Navigate } from 'react-router-dom';
import { AuthGuard } from '../components/AuthGuard';
import { LoginPage } from '../pages/login/index';
import { DashboardPage } from '../pages/dashboard/index';
import { RoomsPage } from '../pages/rooms/index';
import { UsersPage } from '../pages/users/index';
import { LogsPage } from '../pages/logs/index';

export function AppRoutes() {
  return (
    <Routes>
      {/* 根路由：重定向到 /dashboard（AuthGuard 会二次检查认证状态） */}
      <Route path="/" element={<Navigate to="/dashboard" replace />} />

      {/* 公开路由 */}
      <Route path="/login" element={<LoginPage />} />

      {/* 受保护路由 */}
      <Route element={<AuthGuard />}>
        <Route path="/dashboard" element={<DashboardPage />} />
        <Route path="/rooms" element={<RoomsPage />} />
        <Route path="/users" element={<UsersPage />} />
        <Route path="/logs" element={<LogsPage />} />
      </Route>
    </Routes>
  );
}
