/**
 * SkuManagementPage — SKU 管理页 (T-20032)
 *
 * 路由：/payments/skus（RoleGuard: super_admin/operator）
 */

import { useState, useMemo } from 'react';
import { Table, Button, Space, Switch, Select, Tag, Popconfirm, Typography, message } from 'antd';
import type { TableColumnsType } from 'antd';
import { PlusOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useSkuManagementPage } from './useSkuManagementPage';
import { SkuEditModal } from './SkuEditModal';
import { updateSku, deleteSku, type SkuItem } from '../../api/payment';

const { Title } = Typography;

export function SkuManagementPage() {
  const { t } = useTranslation();
  const { items, loading, error, refresh } = useSkuManagementPage();
  const [statusFilter, setStatusFilter] = useState<'all' | 'active' | 'inactive'>('all');
  const [editModalOpen, setEditModalOpen] = useState(false);
  const [editingSku, setEditingSku] = useState<SkuItem | null>(null);
  const [switchingIds, setSwitchingIds] = useState<Set<string>>(new Set());

  const filteredItems = useMemo(() => {
    if (statusFilter === 'all') return items;
    if (statusFilter === 'active') return items.filter((s) => s.is_active);
    return items.filter((s) => !s.is_active);
  }, [items, statusFilter]);

  const handleToggleActive = async (sku: SkuItem, checked: boolean) => {
    setSwitchingIds((prev) => new Set(prev).add(sku.sku_id));
    try {
      await updateSku(sku.sku_id, { is_active: checked });
      refresh();
    } catch (e: unknown) {
      message.error(t('payment.skus.switchError'));
    } finally {
      setSwitchingIds((prev) => {
        const next = new Set(prev);
        next.delete(sku.sku_id);
        return next;
      });
    }
  };

  const handleDelete = async (skuId: string) => {
    try {
      await deleteSku(skuId);
      message.success(t('payment.skus.deleteSuccess'));
      refresh();
    } catch (e: unknown) {
      message.error(e instanceof Error ? e.message : 'Error');
    }
  };

  const columns: TableColumnsType<SkuItem> = [
    {
      title: t('payment.skus.colSkuId'),
      dataIndex: 'sku_id',
      key: 'sku_id',
      width: 150,
    },
    {
      title: t('payment.skus.colProvider'),
      dataIndex: 'provider',
      key: 'provider',
      width: 110,
    },
    {
      title: t('payment.skus.colDiamonds'),
      dataIndex: 'diamonds',
      key: 'diamonds',
      width: 100,
      align: 'right',
    },
    {
      title: t('payment.skus.colPrice'),
      dataIndex: 'display_price_usd',
      key: 'price',
      width: 110,
      align: 'right',
      render: (v: string) => `$${v}`,
    },
    {
      title: t('payment.skus.colSortOrder'),
      dataIndex: 'sort_order',
      key: 'sort_order',
      width: 80,
      align: 'right',
    },
    {
      title: t('payment.skus.colActive'),
      dataIndex: 'is_active',
      key: 'is_active',
      width: 80,
      render: (v: boolean, record) => (
        <Switch
          checked={v}
          loading={switchingIds.has(record.sku_id)}
          onChange={(checked) => handleToggleActive(record, checked)}
        />
      ),
    },
    {
      title: t('payment.skus.colCreatedAt'),
      dataIndex: 'created_at',
      key: 'created_at',
      width: 170,
    },
    {
      title: t('gift.mgmt.colActions'),
      key: 'actions',
      width: 160,
      render: (_, record) => (
        <Space size="small">
          <Button size="small" onClick={() => { setEditingSku(record); setEditModalOpen(true); }}>
            {t('payment.skus.edit')}
          </Button>
          <Popconfirm
            title={t('payment.skus.confirmDelete')}
            onConfirm={() => handleDelete(record.sku_id)}
          >
            <Button size="small" danger>{t('payment.skus.delete')}</Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div data-testid="sku-management-page">
      <Title level={3}>{t('payment.skus.title')}</Title>

      {/* Toolbar */}
      <Space style={{ marginBottom: 16 }}>
        <Select
          value={statusFilter}
          onChange={setStatusFilter}
          style={{ width: 130 }}
          options={[
            { value: 'all', label: t('payment.skus.filterActiveAll') },
            { value: 'active', label: t('payment.skus.filterActive') },
            { value: 'inactive', label: t('payment.skus.filterInactive') },
          ]}
        />
        <Button
          type="primary"
          icon={<PlusOutlined />}
          onClick={() => { setEditingSku(null); setEditModalOpen(true); }}
        >
          {t('payment.skus.add')}
        </Button>
      </Space>

      <Table<SkuItem>
        data-testid="sku-table"
        rowKey="sku_id"
        columns={columns}
        dataSource={filteredItems}
        loading={loading}
        locale={{ emptyText: error ? t('payment.skus.errorLoad') : undefined }}
        pagination={false}
      />

      <SkuEditModal
        open={editModalOpen}
        sku={editingSku}
        onClose={() => { setEditModalOpen(false); setEditingSku(null); }}
        onSuccess={() => { setEditModalOpen(false); setEditingSku(null); refresh(); }}
      />
    </div>
  );
}
