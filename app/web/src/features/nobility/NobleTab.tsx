/**
 * NobleTab — 用户详情中的贵族 Tab (T-20036)
 *
 * 显示当前贵族状态 + 历史记录 + 赠送/撤销操作 (super_admin only)
 */

import { useState, useEffect } from 'react';
import { Descriptions, Table, Tag, Button, Space, Skeleton, Alert, Typography } from 'antd';
import type { TableColumnsType } from 'antd';
import { PlusOutlined, MinusCircleOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import {
  getNobleHistory,
  listNobleUsers,
  type NobleHistoryItem,
  type NobleUserItem,
} from '../../api/nobility';
import { useAuthStore } from '../../stores/useAuthStore';
import { GrantNobleModal } from './GrantNobleModal';
import { RevokeNobleModal } from './GrantNobleModal';

const { Text } = Typography;

const LEVEL_COLORS = ['gold', 'orange', 'purple', 'magenta', 'red', 'volcano'];

interface Props {
  userId: string;
  refreshKey: number;
}

export function NobleTab({ userId, refreshKey }: Props) {
  const { t } = useTranslation();
  const [currentNoble, setCurrentNoble] = useState<NobleUserItem | null>(null);
  const [history, setHistory] = useState<NobleHistoryItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [grantOpen, setGrantOpen] = useState(false);
  const [revokeOpen, setRevokeOpen] = useState(false);
  const [localRefresh, setLocalRefresh] = useState(0);

  const role = useAuthStore((s) => s.admin?.role ?? '');
  const isSuperAdmin = role === 'super_admin';

  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    setError(null);

    Promise.all([
      listNobleUsers({}, controller.signal).catch(() => ({ items: [], total: 0, page: 1, size: 20 })),
      getNobleHistory(userId, controller.signal).catch(() => [] as NobleHistoryItem[]),
    ])
      .then(([usersResult, historyResult]) => {
        const found = usersResult.items.find((u) => u.user_id === userId);
        setCurrentNoble(found ?? null);
        setHistory(historyResult);
      })
      .catch((e: unknown) => {
        if (e instanceof DOMException && e.name === 'AbortError') return;
        setError(e instanceof Error ? e.message : 'Unknown error');
      })
      .finally(() => setLoading(false));

    return () => controller.abort();
  }, [userId, refreshKey, localRefresh]);

  const handleGrantSuccess = () => {
    setGrantOpen(false);
    setLocalRefresh((k) => k + 1);
  };

  const handleRevokeSuccess = () => {
    setRevokeOpen(false);
    setLocalRefresh((k) => k + 1);
  };

  const historyColumns: TableColumnsType<NobleHistoryItem> = [
    {
      title: t('nobility.users.history.colEvent'),
      dataIndex: 'event',
      key: 'event',
      width: 140,
    },
    {
      title: t('nobility.users.history.colFromTier'),
      dataIndex: 'from_tier',
      key: 'from_tier',
      width: 120,
      render: (v: string | null) => v ?? '-',
    },
    {
      title: t('nobility.users.history.colToTier'),
      dataIndex: 'to_tier',
      key: 'to_tier',
      width: 120,
      render: (v: string | null) => v ?? '-',
    },
    {
      title: t('nobility.users.history.colActor'),
      dataIndex: 'actor',
      key: 'actor',
      width: 140,
    },
    {
      title: t('nobility.users.history.colCreatedAt'),
      dataIndex: 'created_at',
      key: 'created_at',
      width: 170,
    },
  ];

  if (loading) return <Skeleton active />;
  if (error) return <Alert type="error" message={error} />;

  return (
    <div data-testid="noble-tab">
      {/* Current Noble Status */}
      {currentNoble ? (
        <>
          <Descriptions column={3} size="small" bordered style={{ marginBottom: 16 }}>
            <Descriptions.Item label={t('nobility.users.colTierName')}>
              <Tag color={LEVEL_COLORS[(currentNoble.tier_level - 1) % 6]}>
                {currentNoble.tier_name_en} (Lv.{currentNoble.tier_level})
              </Tag>
            </Descriptions.Item>
            <Descriptions.Item label={t('nobility.users.colAutoRenew')}>
              {currentNoble.auto_renew ? '✅' : '❌'}
            </Descriptions.Item>
            <Descriptions.Item label="Renew Channel">
              {currentNoble.renew_channel}
            </Descriptions.Item>
            <Descriptions.Item label={t('nobility.users.colStartAt')}>
              {currentNoble.start_at}
            </Descriptions.Item>
            <Descriptions.Item label={t('nobility.users.colExpireAt')}>
              {currentNoble.expire_at}
            </Descriptions.Item>
            <Descriptions.Item label={t('nobility.users.colUserId')}>
              <Text code>{currentNoble.user_id}</Text>
            </Descriptions.Item>
          </Descriptions>

          {isSuperAdmin && (
            <Space style={{ marginBottom: 16 }}>
              <Button
                icon={<PlusOutlined />}
                onClick={() => setGrantOpen(true)}
                data-testid="noble-grant-btn"
              >
                {t('nobility.users.grant.button')}
              </Button>
              <Button
                icon={<MinusCircleOutlined />}
                danger
                onClick={() => setRevokeOpen(true)}
                data-testid="noble-revoke-btn"
              >
                {t('nobility.users.revoke.button')}
              </Button>
            </Space>
          )}
        </>
      ) : (
        <div style={{ marginBottom: 16 }}>
          <Text type="secondary">No nobility</Text>
          {isSuperAdmin && (
            <Button
              type="primary"
              icon={<PlusOutlined />}
              onClick={() => setGrantOpen(true)}
              style={{ marginLeft: 16 }}
              data-testid="noble-grant-btn"
            >
              {t('nobility.users.grant.button')}
            </Button>
          )}
        </div>
      )}

      {/* History */}
      <Text strong>{t('nobility.users.history.title')}</Text>
      <Table<NobleHistoryItem>
        data-testid="noble-history-table"
        rowKey={(_, i) => String(i)}
        columns={historyColumns}
        dataSource={history}
        size="small"
        pagination={false}
        style={{ marginTop: 8 }}
        locale={{ emptyText: t('nobility.users.history.empty') }}
      />

      {/* Modals */}
      <GrantNobleModal
        open={grantOpen}
        userId={userId}
        onClose={() => setGrantOpen(false)}
        onSuccess={handleGrantSuccess}
      />
      <RevokeNobleModal
        open={revokeOpen}
        userId={userId}
        onClose={() => setRevokeOpen(false)}
        onSuccess={handleRevokeSuccess}
      />
    </div>
  );
}
