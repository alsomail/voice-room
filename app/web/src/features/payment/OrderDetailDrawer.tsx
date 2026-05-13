/**
 * OrderDetailDrawer — 订单详情抽屉 (T-20030)
 *
 * open 由 orderId 控制：非 null 时拉取详情并展示。
 */

import { useState, useEffect } from 'react';
import { Drawer, Descriptions, Tag, Skeleton, Alert, Typography } from 'antd';
import { useTranslation } from 'react-i18next';
import { getPaymentOrderDetail, type OrderDetail } from '../../api/payment';

const { Text } = Typography;

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

interface Props {
  orderId: string | null;
  onClose: () => void;
}

export function OrderDetailDrawer({ orderId, onClose }: Props) {
  const { t } = useTranslation();
  const [detail, setDetail] = useState<OrderDetail | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!orderId) {
      setDetail(null);
      setError(null);
      return;
    }

    const controller = new AbortController();
    setLoading(true);
    setError(null);

    getPaymentOrderDetail(orderId, controller.signal)
      .then(setDetail)
      .catch((e: unknown) => {
        if (e instanceof DOMException && e.name === 'AbortError') return;
        setError(e instanceof Error ? e.message : 'Unknown error');
      })
      .finally(() => setLoading(false));

    return () => controller.abort();
  }, [orderId]);

  const stateKey = detail?.state
    ? `payment.orders.state${detail.state.charAt(0) + detail.state.slice(1).toLowerCase()}`
    : '';

  return (
    <Drawer
      data-testid="order-detail-drawer"
      title={t('payment.orders.detail.title')}
      open={!!orderId}
      onClose={onClose}
      width={560}
      destroyOnHidden
    >
      {loading && <Skeleton active data-testid="order-detail-skeleton" />}
      {error && <Alert type="error" message={error} data-testid="order-detail-error" />}
      {!loading && !error && detail && (
        <>
          <Descriptions column={2} size="small" bordered style={{ marginBottom: 16 }}>
            <Descriptions.Item label={t('payment.orders.detail.orderId')} span={2}>
              <Text code>{detail.order_id}</Text>
            </Descriptions.Item>
            <Descriptions.Item label={t('payment.orders.detail.userId')}>
              <Text code>{detail.user_id}</Text>
            </Descriptions.Item>
            <Descriptions.Item label={t('payment.orders.detail.skuId')}>
              {detail.sku_id}
            </Descriptions.Item>
            <Descriptions.Item label={t('payment.orders.detail.state')}>
              <Tag color={STATE_COLORS[detail.state] ?? 'default'}>
                {t(stateKey as never, detail.state)}
              </Tag>
            </Descriptions.Item>
            <Descriptions.Item label={t('payment.orders.detail.provider')}>
              {detail.provider}
            </Descriptions.Item>
            <Descriptions.Item label={t('payment.orders.detail.amountMicros')}>
              {detail.amount_micros != null ? `$${(detail.amount_micros / 1_000_000).toFixed(2)}` : '-'}
            </Descriptions.Item>
            <Descriptions.Item label={t('payment.orders.detail.currency')}>
              {detail.currency ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label={t('payment.orders.detail.countryCode')}>
              {detail.country_code ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label={t('payment.orders.detail.purchaseToken')}>
              <Text code>{detail.purchase_token_masked ?? '-'}</Text>
            </Descriptions.Item>
            <Descriptions.Item label={t('payment.orders.detail.providerOrderId')}>
              {detail.provider_order_id ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label={t('payment.orders.detail.createdAt')}>
              {detail.created_at}
            </Descriptions.Item>
            <Descriptions.Item label={t('payment.orders.detail.verifiedAt')}>
              {detail.verified_at ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label={t('payment.orders.detail.creditedAt')}>
              {detail.credited_at ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label={t('payment.orders.detail.ackedAt')}>
              {detail.acked_at ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label={t('payment.orders.detail.failedAt')}>
              {detail.failed_at ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label={t('payment.orders.detail.failedReason')} span={2}>
              {detail.failed_reason ?? '-'}
            </Descriptions.Item>
          </Descriptions>

          {/* Risk Flags */}
          {detail.risk_flags.length > 0 && (
            <div style={{ marginBottom: 16 }}>
              <Text strong>{t('payment.orders.detail.riskFlags')}:</Text>
              <div>
                {detail.risk_flags.map((f) => (
                  <Tag key={f} color="orange">{f}</Tag>
                ))}
              </div>
            </div>
          )}

          {/* State History */}
          <div style={{ marginBottom: 16 }}>
            <Text strong>{t('payment.orders.detail.stateHistory')}:</Text>
            <pre style={{
              background: '#f5f5f5',
              padding: 8,
              borderRadius: 4,
              maxHeight: 200,
              overflow: 'auto',
              fontSize: 12,
            }}>
              {JSON.stringify(detail.state_history, null, 2)}
            </pre>
          </div>

          {/* Google Raw Response */}
          {detail.provider_response_raw && (
            <div>
              <Text strong>{t('payment.orders.detail.providerResponse')}:</Text>
              <pre style={{
                background: '#f5f5f5',
                padding: 8,
                borderRadius: 4,
                maxHeight: 300,
                overflow: 'auto',
                fontSize: 12,
              }}>
                {JSON.stringify(detail.provider_response_raw, null, 2)}
              </pre>
            </div>
          )}
        </>
      )}
    </Drawer>
  );
}
