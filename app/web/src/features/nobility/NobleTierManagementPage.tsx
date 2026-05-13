/**
 * NobleTierManagementPage — 贵族等级管理页 (T-20035)
 *
 * 路由：/nobles/tiers（RoleGuard: super_admin/operator）
 */

import { useState, useMemo } from 'react';
import { Table, Button, Space, Select, Switch, Popconfirm, Tag, Typography, message } from 'antd';
import type { TableColumnsType } from 'antd';
import { PlusOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useNobleTierManagementPage } from './useNobleTierManagementPage';
import { NobleTierEditModal } from './NobleTierEditModal';
import { updateNobleTier, deleteNobleTier, type TierItem } from '../../api/nobility';

const { Title } = Typography;

const LEVEL_COLORS = ['gold', 'orange', 'purple', 'magenta', 'red', 'volcano'];

export function NobleTierManagementPage() {
  const { t } = useTranslation();
  const { items, total, loading, error, page, pageSize, setPage, setPageSize, refresh } =
    useNobleTierManagementPage();
  const [statusFilter, setStatusFilter] = useState<'all' | 'active' | 'inactive'>('all');
  const [editModalOpen, setEditModalOpen] = useState(false);
  const [editingTier, setEditingTier] = useState<TierItem | null>(null);
  const [switchingIds, setSwitchingIds] = useState<Set<string>>(new Set());

  const filteredItems = useMemo(() => {
    if (statusFilter === 'all') return items;
    if (statusFilter === 'active') return items.filter((t) => t.is_active);
    return items.filter((t) => !t.is_active);
  }, [items, statusFilter]);

  const handleToggleActive = async (tier: TierItem, checked: boolean) => {
    setSwitchingIds((prev) => new Set(prev).add(tier.tier_id));
    try {
      await updateNobleTier(tier.tier_id, { is_active: checked } as never);
      refresh();
    } catch (e: unknown) {
      message.error(t('nobility.tiers.switchError'));
    } finally {
      setSwitchingIds((prev) => {
        const next = new Set(prev);
        next.delete(tier.tier_id);
        return next;
      });
    }
  };

  const handleDelete = async (tierId: string) => {
    try {
      await deleteNobleTier(tierId);
      message.success(t('nobility.tiers.deleteSuccess'));
      refresh();
    } catch (e: unknown) {
      message.error(e instanceof Error ? e.message : 'Error');
    }
  };

  const columns: TableColumnsType<TierItem> = [
    {
      title: t('nobility.tiers.colTierId'),
      dataIndex: 'tier_id',
      key: 'tier_id',
      width: 130,
    },
    {
      title: t('nobility.tiers.colLevel'),
      dataIndex: 'level',
      key: 'level',
      width: 70,
      align: 'center',
      render: (v: number) => (
        <Tag color={LEVEL_COLORS[(v - 1) % 6]}>{v}</Tag>
      ),
    },
    {
      title: t('nobility.tiers.colName'),
      dataIndex: 'name_en',
      key: 'name',
      width: 160,
      render: (v: string, r: TierItem) => `${v} / ${r.name_ar}`,
    },
    {
      title: t('nobility.tiers.colDiamonds'),
      dataIndex: 'monthly_diamonds',
      key: 'diamonds',
      width: 130,
      align: 'right',
    },
    {
      title: t('nobility.tiers.colPrice'),
      dataIndex: 'monthly_usd',
      key: 'price',
      width: 120,
      align: 'right',
      render: (v: string) => `$${v}`,
    },
    {
      title: t('nobility.tiers.colActive'),
      dataIndex: 'is_active',
      key: 'is_active',
      width: 80,
      render: (v: boolean, record) => (
        <Switch
          checked={v}
          loading={switchingIds.has(record.tier_id)}
          onChange={(checked) => handleToggleActive(record, checked)}
        />
      ),
    },
    {
      title: t('gift.mgmt.colActions'),
      key: 'actions',
      width: 160,
      render: (_, record) => (
        <Space size="small">
          <Button
            size="small"
            onClick={() => { setEditingTier(record); setEditModalOpen(true); }}
          >
            {t('nobility.tiers.edit')}
          </Button>
          <Popconfirm
            title={t('nobility.tiers.confirmDelete')}
            onConfirm={() => handleDelete(record.tier_id)}
          >
            <Button size="small" danger>{t('nobility.tiers.delete')}</Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div data-testid="noble-tier-management-page">
      <Title level={3}>{t('nobility.tiers.title')}</Title>

      <Space style={{ marginBottom: 16 }}>
        <Select
          value={statusFilter}
          onChange={setStatusFilter}
          style={{ width: 130 }}
          options={[
            { value: 'all', label: t('nobility.tiers.filterActiveAll') },
            { value: 'active', label: t('nobility.tiers.filterActive') },
            { value: 'inactive', label: t('nobility.tiers.filterInactive') },
          ]}
        />
        <Button
          type="primary"
          icon={<PlusOutlined />}
          onClick={() => { setEditingTier(null); setEditModalOpen(true); }}
        >
          {t('nobility.tiers.add')}
        </Button>
      </Space>

      <Table<TierItem>
        data-testid="noble-tier-table"
        rowKey="tier_id"
        columns={columns}
        dataSource={filteredItems}
        loading={loading}
        pagination={{
          current: page,
          pageSize,
          total,
          showSizeChanger: true,
          onChange: (p, ps) => { setPage(p); setPageSize(ps); },
        }}
        locale={{ emptyText: error ? t('nobility.tiers.errorLoad') : undefined }}
      />

      <NobleTierEditModal
        open={editModalOpen}
        tier={editingTier}
        onClose={() => { setEditModalOpen(false); setEditingTier(null); }}
        onSuccess={() => { setEditModalOpen(false); setEditingTier(null); refresh(); }}
      />
    </div>
  );
}
