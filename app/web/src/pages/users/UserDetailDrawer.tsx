/**
 * UserDetailDrawer — 用户详情抽屉组件（T-20007）
 *
 * Props：
 *   userId        — 当前选中的用户 ID（null 时 Drawer 关闭）
 *   onClose       — Drawer 关闭回调
 *   onBanClick    — 封禁按钮点击回调（传入 userId）
 *   onUnbanClick  — 解封按钮点击回调（传入 userId）
 *
 * T-20012 新增：
 *   - "调整余额"按钮，点击打开 AdjustBalanceModal
 *   - 调整成功后余额实时刷新（通过 refreshKey 触发 useUserDetail 重新拉取）
 *
 * 内部结构：
 *   - 调用 useUserDetail(userId) 拉取用户详情
 *   - loading → <Skeleton data-testid="detail-skeleton" />
 *   - error   → <Alert data-testid="detail-error" />
 *   - detail  → <Descriptions> 基础信息 + <Statistic> 金币余额
 *   - 行为数据占位：data-testid="behavior-placeholder"
 *   - 操作区：status==='banned' → [解封] 按钮；否则 → [封禁] 按钮 + [调整余额] 按钮
 *   - open={!!userId}，destroyOnHidden
 */

import { useState } from 'react';
import { Drawer, Skeleton, Alert, Descriptions, Statistic, Button, Space, Avatar } from 'antd';
import { useTranslation } from 'react-i18next';
import { useUserDetail } from './useUserDetail';
import { UserStatusTag } from './UserStatusTag';
import { AdjustBalanceModal } from '../../features/user/AdjustBalanceModal';
import { useAuthStore } from '../../stores/useAuthStore';

/** 有权限调整余额的角色（对应 T-10013 RBAC WalletAdjust 权限） */
const WALLET_ADJUST_ROLES = ['super_admin', 'operator', 'finance'];

export interface UserDetailDrawerProps {
  userId: string | null;
  onClose: () => void;
  onBanClick?: (userId: string) => void;
  onUnbanClick?: (userId: string) => void;
}

export function UserDetailDrawer({
  userId,
  onClose,
  onBanClick,
  onUnbanClick,
}: UserDetailDrawerProps) {
  const { t } = useTranslation();
  const [adjustOpen, setAdjustOpen] = useState(false);
  // refreshKey 变化触发 useUserDetail 重新拉取
  const [refreshKey, setRefreshKey] = useState(0);
  const { detail, loading, error } = useUserDetail(userId, refreshKey);

  // MEDIUM-2 修复：根据角色控制"调整余额"按钮可见性（W12-01 RBAC）
  const admin = useAuthStore((s) => s.admin);
  const role = admin?.role ?? '';
  const canAdjustBalance = WALLET_ADJUST_ROLES.includes(role);

  // LOW 修复：省略 newBalance 参数（TypeScript 允许回调省略尾部参数），通过 refreshKey 重新拉取最新余额
  const handleAdjustSuccess = () => {
    // 通过 refreshKey 重新拉取最新余额，无需直接使用 newBalance
    setRefreshKey((k) => k + 1);
    setAdjustOpen(false);
  };

  return (
    <>
      <Drawer
        open={!!userId}
        title={t('users.drawer.title')}
        onClose={onClose}
        destroyOnHidden
        width={480}
      >
        {/* 加载态 */}
        {loading && (
          <div data-testid="detail-skeleton">
            <Skeleton active paragraph={{ rows: 6 }} />
          </div>
        )}

        {/* 错误态 */}
        {error && !loading && (
          <Alert
            data-testid="detail-error"
            type="error"
            message={t('users.drawer.errorTitle')}
            description={error.message}
            showIcon
            style={{ marginBottom: 16 }}
          />
        )}

        {/* 成功态：基础信息 + 资产信息 + 操作区 */}
        {detail && !loading && (
          <>
            {/* 基础信息 */}
            <Descriptions
              title={t('users.drawer.basicInfo')}
              column={1}
              size="small"
              style={{ marginBottom: 24 }}
            >
              <Descriptions.Item label={t('users.drawer.avatar')}>
                <Avatar src={detail.avatar_url ?? undefined} size={40} />
              </Descriptions.Item>
              <Descriptions.Item label={t('users.drawer.phone')}>
                {detail.phone}
              </Descriptions.Item>
              <Descriptions.Item label={t('users.drawer.nickname')}>
                {detail.nickname}
              </Descriptions.Item>
              <Descriptions.Item label={t('users.drawer.status')}>
                <UserStatusTag status={detail.status} />
              </Descriptions.Item>
              <Descriptions.Item label={t('users.drawer.createdAt')}>
                {new Date(detail.created_at).toLocaleString()}
              </Descriptions.Item>
            </Descriptions>

            {/* 资产信息 */}
            <div style={{ marginBottom: 24 }}>
              <Statistic
                data-testid="coin-balance-stat"
                title={t('users.drawer.coinBalance')}
                value={detail.coin_balance}
              />
            </div>

            {/* 行为数据占位 */}
            <div data-testid="behavior-placeholder" style={{ marginBottom: 24 }}>
              <strong>{t('users.drawer.behaviorData')}</strong>
              <p style={{ color: '#999', marginTop: 4 }}>
                {t('users.drawer.behaviorPlaceholder')}
              </p>
            </div>

            {/* 操作区 */}
            <Space>
              {detail.status === 'banned' ? (
                <Button
                  data-testid="unban-btn"
                  type="primary"
                  onClick={() => onUnbanClick?.(detail.id)}
                >
                  {t('users.drawer.unban')}
                </Button>
              ) : (
                <Button
                  data-testid="ban-btn"
                  danger
                  onClick={() => onBanClick?.(detail.id)}
                >
                  {t('users.drawer.ban')}
                </Button>
              )}

              {/* T-20012: 调整余额按钮（仅 super_admin/operator/finance 可见） */}
              {canAdjustBalance && (
                <Button
                  data-testid="adjust-balance-btn"
                  onClick={() => setAdjustOpen(true)}
                >
                  {t('wallet.adjust.adjustBalance')}
                </Button>
              )}
            </Space>
          </>
        )}
      </Drawer>

      {/* 调整余额弹窗 */}
      {detail && userId && (
        <AdjustBalanceModal
          userId={userId}
          currentBalance={detail.coin_balance}
          open={adjustOpen}
          onClose={() => setAdjustOpen(false)}
          onSuccess={handleAdjustSuccess}
        />
      )}
    </>
  );
}

