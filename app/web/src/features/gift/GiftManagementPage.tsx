/**
 * GiftManagementPage — 礼物管理页（T-20012）
 *
 * 路由：/gifts（受 AuthGuard 保护，需 super_admin/operator 角色）
 *
 * 功能：
 *   - 礼物列表（Table，含 tier/状态筛选）
 *   - Switch 上下架（乐观更新 + 失败回滚）
 *   - 新增礼物（GiftEditModal）
 *   - 编辑礼物（GiftEditModal）
 *   - 软删除（Popconfirm 二次确认）
 *
 * 筛选：
 *   - tier 下拉（全部/1-5）
 *   - 状态下拉（全部/已上架/已下架）→ include_inactive 参数
 */

import { useState, useEffect, useCallback } from 'react';
import {
  Table,
  Button,
  Switch,
  Space,
  Select,
  Popconfirm,
  Alert,
  Image,
  message,
  Typography,
} from 'antd';
import type { TableColumnsType } from 'antd';
import { PlusOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import {
  adminListGifts,
  adminUpdateGift,
  adminDeleteGift,
  type AdminGiftItem,
  type AdminListGiftsParams,
} from '../../core/network/apiClient';
import { GiftEditModal } from './GiftEditModal';

const { Title } = Typography;

export function GiftManagementPage() {
  const { t } = useTranslation();

  // ── 列表状态
  const [gifts, setGifts] = useState<AdminGiftItem[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  // ── 筛选状态
  const [tierFilter, setTierFilter] = useState<number | undefined>(undefined);
  const [statusFilter, setStatusFilter] = useState<'all' | 'active' | 'inactive'>('all');
  const [page, setPage] = useState(1);

  // ── 编辑弹窗状态
  const [editModalOpen, setEditModalOpen] = useState(false);
  const [editingGift, setEditingGift] = useState<AdminGiftItem | null>(null);

  // ── 拉取礼物列表
  const fetchGifts = useCallback(async (
    params: AdminListGiftsParams,
    clientFilter: 'all' | 'active' | 'inactive' = 'all',
    signal?: AbortSignal,
  ) => {
    setLoading(true);
    setLoadError(null);
    try {
      const result = await adminListGifts(params, signal);
      // HIGH-1 修复：inactive 筛选在前端做客户端过滤
      // API 不支持"仅返回下架礼物"，需先以 include_inactive=true 获取全量再过滤
      const items =
        clientFilter === 'inactive'
          ? result.items.filter((g) => !g.is_active)
          : result.items;
      setGifts(items);
      setTotal(items.length);
    } catch (err) {
      if ((err as Error).name !== 'AbortError') {
        setLoadError(err instanceof Error ? err.message : t('gift.mgmt.errorLoad'));
      }
    } finally {
      setLoading(false);
    }
  }, [t]);

  // ── 筛选变化时重新加载（含 AbortController 防竞态）
  useEffect(() => {
    const controller = new AbortController();
    const params: AdminListGiftsParams = {
      page,
      size: 50,
    };
    if (tierFilter !== undefined) params.tier = tierFilter;
    // 'all' 和 'inactive' 均需要 include_inactive=true 以获取全量数据；
    // 'inactive' 的下架过滤由 fetchGifts 的客户端过滤完成。
    params.include_inactive = statusFilter !== 'active';
    void fetchGifts(params, statusFilter, controller.signal);
    return () => controller.abort();
  }, [page, tierFilter, statusFilter, fetchGifts]);

  // ── Switch 切换上下架（乐观更新 + 失败回滚）
  const handleToggleActive = async (id: string, currentActive: boolean) => {
    const newActive = !currentActive;
    // 乐观更新
    setGifts((prev) =>
      prev.map((g) => (g.id === id ? { ...g, is_active: newActive } : g)),
    );
    try {
      await adminUpdateGift(id, { is_active: newActive });
    } catch {
      // 回滚
      setGifts((prev) =>
        prev.map((g) => (g.id === id ? { ...g, is_active: currentActive } : g)),
      );
      void message.error(t('gift.mgmt.switchError'));
    }
  };

  // ── 软删除
  const handleDelete = async (id: string) => {
    try {
      await adminDeleteGift(id);
      void message.success(t('gift.mgmt.deleteSuccess'));
      // 刷新列表
      setGifts((prev) => prev.filter((g) => g.id !== id));
      setTotal((prev) => prev - 1);
    } catch (err) {
      void message.error(err instanceof Error ? err.message : t('common.requestError'));
    }
  };

  // ── 打开新增弹窗
  const handleAdd = () => {
    setEditingGift(null);
    setEditModalOpen(true);
  };

  // ── 打开编辑弹窗
  const handleEdit = (gift: AdminGiftItem) => {
    setEditingGift(gift);
    setEditModalOpen(true);
  };

  // ── 编辑成功后刷新
  const handleEditSuccess = (isCreate: boolean) => {
    void message.success(isCreate ? t('gift.mgmt.createSuccess') : t('gift.mgmt.updateSuccess'));
    void fetchGifts({
      page,
      size: 50,
      include_inactive: statusFilter !== 'active',
      ...(tierFilter !== undefined ? { tier: tierFilter } : {}),
    }, statusFilter);
  };

  // ── tier 筛选选项
  const tierOptions = [
    { label: t('gift.mgmt.filterTierAll'), value: 0 },
    { label: t('gift.mgmt.tier1'), value: 1 },
    { label: t('gift.mgmt.tier2'), value: 2 },
    { label: t('gift.mgmt.tier3'), value: 3 },
    { label: t('gift.mgmt.tier4'), value: 4 },
    { label: t('gift.mgmt.tier5'), value: 5 },
  ];

  // ── 状态筛选选项
  const statusOptions = [
    { label: t('gift.mgmt.filterStatusAll'), value: 'all' },
    { label: t('gift.mgmt.filterActive'), value: 'active' },
    { label: t('gift.mgmt.filterInactive'), value: 'inactive' },
  ];

  // ── 表格列定义
  const columns: TableColumnsType<AdminGiftItem> = [
    {
      title: t('gift.mgmt.colIcon'),
      key: 'icon',
      width: 64,
      render: (_, record) => (
        <Image
          src={record.icon_url}
          width={40}
          height={40}
          style={{ objectFit: 'contain' }}
          preview={false}
          fallback="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
        />
      ),
    },
    {
      title: t('gift.mgmt.colCode'),
      dataIndex: 'code',
      key: 'code',
    },
    {
      title: t('gift.mgmt.colName'),
      key: 'name',
      render: (_, record) => `${record.name_en} / ${record.name_ar}`,
    },
    {
      title: t('gift.mgmt.colPrice'),
      dataIndex: 'price',
      key: 'price',
    },
    {
      title: t('gift.mgmt.colTier'),
      dataIndex: 'tier',
      key: 'tier',
    },
    {
      title: t('gift.mgmt.colActive'),
      key: 'is_active',
      render: (_, record) => (
        <Switch
          data-testid={`gift-switch-${record.id}`}
          checked={record.is_active}
          onChange={() => void handleToggleActive(record.id, record.is_active)}
        />
      ),
    },
    {
      title: t('gift.mgmt.colActions'),
      key: 'actions',
      render: (_, record) => (
        <Space>
          <Button
            data-testid={`gift-edit-btn-${record.id}`}
            size="small"
            onClick={() => handleEdit(record)}
          >
            {t('gift.mgmt.edit')}
          </Button>
          <Popconfirm
            title={t('gift.mgmt.confirmDelete')}
            onConfirm={() => void handleDelete(record.id)}
            okText={t('wallet.adjust.submitBtn')}
            cancelText={t('wallet.adjust.cancelBtn')}
          >
            <Button
              data-testid={`gift-delete-btn-${record.id}`}
              size="small"
              danger
            >
              {t('gift.mgmt.delete')}
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div style={{ padding: 24 }}>
      {/* 页面标题 */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: 16,
        }}
      >
        <Title data-testid="gift-page-title" level={4} style={{ margin: 0 }}>
          {t('gift.mgmt.title')}
        </Title>
        <Button
          data-testid="add-gift-btn"
          type="primary"
          icon={<PlusOutlined />}
          onClick={handleAdd}
        >
          {t('gift.mgmt.add')}
        </Button>
      </div>

      {/* 筛选栏 */}
      <Space style={{ marginBottom: 16 }}>
        <Select
          data-testid="gift-tier-filter"
          value={tierFilter ?? 0}
          options={tierOptions}
          onChange={(v: number) => {
            setTierFilter(v === 0 ? undefined : v);
            setPage(1);
          }}
          style={{ width: 120 }}
        />
        <Select
          data-testid="gift-status-filter"
          value={statusFilter}
          options={statusOptions}
          onChange={(v: 'all' | 'active' | 'inactive') => {
            setStatusFilter(v);
            setPage(1);
          }}
          style={{ width: 120 }}
        />
      </Space>

      {/* 错误提示 */}
      {loadError && (
        <Alert
          type="error"
          message={loadError}
          showIcon
          style={{ marginBottom: 16 }}
        />
      )}

      {/* 礼物列表 */}
      <Table<AdminGiftItem>
        dataSource={gifts}
        columns={columns}
        rowKey="id"
        loading={loading}
        pagination={{
          current: page,
          total,
          pageSize: 50,
          onChange: (p) => setPage(p),
          showSizeChanger: false,
        }}
      />

      {/* 新增/编辑弹窗 */}
      <GiftEditModal
        gift={editingGift}
        open={editModalOpen}
        onClose={() => setEditModalOpen(false)}
        onSuccess={handleEditSuccess}
      />
    </div>
  );
}
