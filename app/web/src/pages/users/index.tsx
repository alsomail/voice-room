/**
 * UsersPage — 用户管理页面（T-20006 / T-20007 / T-20008 / T-20010）
 *
 * 入口组件：集成 useUsersPage Hook + UserSearchForm 组件 + UsersTable 组件 + UserDetailDrawer 组件 + BanModal + UnbanModal
 */

import { useState, useCallback } from 'react';
import { Alert, Typography, message } from 'antd';
import { useTranslation } from 'react-i18next';
import { useUsersPage } from './useUsersPage';
import { UserSearchForm } from './UserSearchForm';
import { UsersTable } from './UsersTable';
import { UserDetailDrawer } from './UserDetailDrawer';
import { BanModal } from './BanModal';
import { UnbanModal } from './UnbanModal';

export function UsersPage() {
  const { t } = useTranslation();
  const {
    items,
    total,
    loading,
    error,
    page,
    pageSize,
    filters,
    setPage,
    setFilters,
    refresh,
  } = useUsersPage();

  // T-20007: 选中的用户 ID（null 时抽屉关闭）
  const [selectedUserId, setSelectedUserId] = useState<string | null>(null);

  // T-20008: 正在封禁的用户 ID（null 时 BanModal 关闭）
  const [banUserId, setBanUserId] = useState<string | null>(null);

  // T-20010: 正在解封的用户 ID（null 时 UnbanModal 关闭）
  const [unbanUserId, setUnbanUserId] = useState<string | null>(null);

  const handleReset = useCallback(() => {
    setFilters({});
  }, [setFilters]);

  const handleViewDetail = useCallback((userId: string) => {
    setSelectedUserId(userId);
  }, []);

  const handleDrawerClose = useCallback(() => {
    setSelectedUserId(null);
  }, []);

  // ── T-20008: 封禁流程 ──────────────────────────────────────────────────────
  const handleBanClick = useCallback((userId: string) => {
    setBanUserId(userId);
  }, []);

  const handleBanClose = useCallback(() => {
    setBanUserId(null);
  }, []);

  const handleBanSuccess = useCallback(
    () => {
      setBanUserId(null);
      setSelectedUserId(null); // 关闭 Drawer
      message.success(t('users.ban.successMsg'));
      refresh(); // 刷新用户列表
    },
    [t, refresh],
  );

  // ── T-20010: 解封流程 ──────────────────────────────────────────────────────
  const handleUnbanClick = useCallback((userId: string) => {
    setUnbanUserId(userId);
  }, []);

  const handleUnbanClose = useCallback(() => {
    setUnbanUserId(null);
  }, []);

  const handleUnbanSuccess = useCallback(
    (userId: string) => {
      void userId; // userId 已通过 setUnbanUserId 追踪，此处仅作类型完整性标注
      message.success(t('users.unban.successMsg'));
      setUnbanUserId(null);
      setSelectedUserId(null);
      refresh();
    },
    [t, refresh],
  );

  return (
    <div data-testid="users-page" style={{ padding: '24px' }}>
      <Typography.Title level={4} style={{ marginBottom: 16 }}>
        {t('users.title')}
      </Typography.Title>

      {/* 错误提示 */}
      {error && (
        <Alert
          data-testid="users-error"
          type="error"
          description={error.message}
          showIcon
          style={{ marginBottom: 16 }}
        />
      )}

      {/* 搜索表单 */}
      <UserSearchForm
        initialFilters={filters}
        onSearch={setFilters}
        onReset={handleReset}
      />

      {/* 用户列表 */}
      <UsersTable
        items={items}
        total={total}
        page={page}
        pageSize={pageSize}
        loading={loading}
        onPageChange={setPage}
        onRefresh={refresh}
        onViewDetail={handleViewDetail}
      />

      {/* 用户详情抽屉（T-20007） */}
      <UserDetailDrawer
        userId={selectedUserId}
        onClose={handleDrawerClose}
        onBanClick={handleBanClick}
        onUnbanClick={handleUnbanClick}
      />

      {/* 封禁对话框（T-20008） */}
      <BanModal
        userId={banUserId}
        onClose={handleBanClose}
        onSuccess={handleBanSuccess}
      />

      {/* 解封对话框（T-20010） */}
      <UnbanModal
        userId={unbanUserId}
        onClose={handleUnbanClose}
        onSuccess={handleUnbanSuccess}
      />
    </div>
  );
}
