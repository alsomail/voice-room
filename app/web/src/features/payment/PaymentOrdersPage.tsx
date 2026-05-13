/**
 * PaymentOrdersPage — 支付订单列表页 (T-20030)
 *
 * 路由：/payments/orders（RoleGuard: super_admin/operator/finance）
 */

import { useState } from 'react';
import { Table, Button, Space, Input, Select, DatePicker, Tag, Typography } from 'antd';
import type { TableColumnsType } from 'antd';
import { SearchOutlined, ReloadOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { usePaymentOrdersPage } from './usePaymentOrdersPage';
import { OrderDetailDrawer } from './OrderDetailDrawer';
import { RecreditRefundModal } from './RecreditRefundModal';
import { useAuthStore } from '../../stores/useAuthStore';
import type { PaymentOrderListItem } from '../../api/payment';

const { Title } = Typography;
const { RangePicker } = DatePicker;

const ORDER_STATES = [
  'PENDING', 'VERIFYING', 'VERIFIED', 'CREDITED', 'ACKED',
  'CANCELLED', 'FAILED', 'REFUNDED',
];

const STATE_COLORS: Record<string, string> = {
  PENDING: 'default',
  VERIFYING: 'processing',
  VERIFIED: 'cyan',
  CREDITED: 'green',
  ACKED: 'success',
  CANCELLED: 'warning',
  FAILED: 'error',
  REFUNDED: 'red',
};

export function PaymentOrdersPage() {
  const { t } = useTranslation();
  const {
    items, total, loading, error,
    page, pageSize, setPage, setPageSize,
    applyFilters, resetFilters, refresh,
  } = usePaymentOrdersPage();

  const [detailOrderId, setDetailOrderId] = useState<string | null>(null);
  const [recreditOrder, setRecreditOrder] = useState<PaymentOrderListItem | null>(null);
  const [refundOrder, setRefundOrder] = useState<PaymentOrderListItem | null>(null);

  // Local filter state
  const [userId, setUserId] = useState('');
  const [state, setState] = useState<string | undefined>(undefined);
  const [provider, setProvider] = useState<string | undefined>(undefined);
  const [dateRange, setDateRange] = useState<[string, string] | null>(null);

  const role = useAuthStore((s) => s.admin?.role ?? '');
  const isSuperAdmin = role === 'super_admin';

  const handleSearch = () => {
    applyFilters({
      user_id: userId || undefined,
      state,
      provider,
      created_from: dateRange?.[0],
      created_to: dateRange?.[1],
    });
  };

  const handleReset = () => {
    setUserId('');
    setState(undefined);
    setProvider(undefined);
    setDateRange(null);
    resetFilters();
  };

  const columns: TableColumnsType<PaymentOrderListItem> = [
    {
      title: t('payment.orders.colOrderId'),
      dataIndex: 'order_id',
      key: 'order_id',
      width: 180,
      render: (v: string) => <span style={{ fontFamily: 'monospace', fontSize: 12 }}>{v.slice(0, 8)}...</span>,
    },
    {
      title: t('payment.orders.colUserId'),
      dataIndex: 'user_id',
      key: 'user_id',
      width: 180,
      render: (v: string) => <span style={{ fontFamily: 'monospace', fontSize: 12 }}>{v.slice(0, 8)}...</span>,
    },
    {
      title: t('payment.orders.colSkuId'),
      dataIndex: 'sku_id',
      key: 'sku_id',
      width: 140,
    },
    {
      title: t('payment.orders.colState'),
      dataIndex: 'state',
      key: 'state',
      width: 110,
      render: (s: string) => (
        <Tag color={STATE_COLORS[s] ?? 'default'} data-testid={`order-state-${s}`}>
          {t(`payment.orders.state${s.charAt(0) + s.slice(1).toLowerCase()}` as never, s)}
        </Tag>
      ),
    },
    {
      title: t('payment.orders.colProvider'),
      dataIndex: 'provider',
      key: 'provider',
      width: 110,
    },
    {
      title: t('payment.orders.colAmount'),
      dataIndex: 'amount_micros',
      key: 'amount',
      width: 100,
      align: 'right',
      render: (v: number | null) =>
        v != null ? `$${(v / 1_000_000).toFixed(2)}` : '-',
    },
    {
      title: t('payment.orders.colCreatedAt'),
      dataIndex: 'created_at',
      key: 'created_at',
      width: 170,
    },
    {
      title: t('gift.mgmt.colActions'),
      key: 'actions',
      width: 200,
      render: (_, record) => (
        <Space size="small">
          <Button size="small" onClick={() => setDetailOrderId(record.order_id)}>
            {t('payment.orders.viewDetail')}
          </Button>
          {isSuperAdmin && (
            <>
              <Button size="small" onClick={() => setRecreditOrder(record)}>
                {t('payment.orders.detail.recredit')}
              </Button>
              <Button size="small" danger onClick={() => setRefundOrder(record)}>
                {t('payment.orders.detail.refund')}
              </Button>
            </>
          )}
        </Space>
      ),
    },
  ];

  return (
    <div data-testid="payment-orders-page">
      <Title level={3}>{t('payment.orders.title')}</Title>

      {/* Search Bar */}
      <Space wrap style={{ marginBottom: 16 }}>
        <Input
          placeholder={t('payment.orders.filterUserId')}
          value={userId}
          onChange={(e) => setUserId(e.target.value)}
          style={{ width: 260 }}
          allowClear
        />
        <Select
          placeholder={t('payment.orders.filterState')}
          value={state}
          onChange={setState}
          allowClear
          style={{ width: 140 }}
          options={[
            { value: '', label: t('payment.orders.filterStateAll') },
            ...ORDER_STATES.map((s) => ({
              value: s,
              label: t(`payment.orders.state${s.charAt(0) + s.slice(1).toLowerCase()}` as never, s),
            })),
          ]}
        />
        <Select
          placeholder={t('payment.orders.filterProvider')}
          value={provider}
          onChange={setProvider}
          allowClear
          style={{ width: 140 }}
          options={[
            { value: '', label: t('payment.orders.filterProviderAll') },
            { value: 'google_play', label: 'Google Play' },
            { value: 'mock', label: 'Mock' },
          ]}
        />
        <RangePicker
          onChange={(_, dateStrings) =>
            setDateRange(dateStrings[0] && dateStrings[1] ? [dateStrings[0], dateStrings[1]] : null)
          }
        />
        <Button type="primary" icon={<SearchOutlined />} onClick={handleSearch}>
          {t('payment.orders.search')}
        </Button>
        <Button icon={<ReloadOutlined />} onClick={handleReset}>
          {t('payment.orders.reset')}
        </Button>
      </Space>

      {/* Table */}
      <Table<PaymentOrderListItem>
        data-testid="payment-orders-table"
        rowKey="order_id"
        columns={columns}
        dataSource={items}
        loading={loading}
        pagination={{
          current: page,
          pageSize,
          total,
          showSizeChanger: true,
          onChange: (p, ps) => { setPage(p); setPageSize(ps); },
        }}
        locale={{ emptyText: error ? t('payment.orders.errorLoad') : undefined }}
      />

      {/* Detail Drawer */}
      <OrderDetailDrawer
        orderId={detailOrderId}
        onClose={() => setDetailOrderId(null)}
      />

      {/* Recredit Modal */}
      <RecreditRefundModal
        type="recredit"
        order={recreditOrder}
        onClose={() => setRecreditOrder(null)}
        onSuccess={() => { setRecreditOrder(null); refresh(); }}
      />

      {/* Refund Modal */}
      <RecreditRefundModal
        type="refund"
        order={refundOrder}
        onClose={() => setRefundOrder(null)}
        onSuccess={() => { setRefundOrder(null); refresh(); }}
      />
    </div>
  );
}
