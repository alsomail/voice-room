/**
 * RecreditRefundModal — 补单/退款弹窗 (T-20031)
 *
 * super_admin only。需输入 CONFIRM 二次确认。
 */

import { useState } from 'react';
import { Modal, Input, Alert, Typography, Result } from 'antd';
import { useTranslation } from 'react-i18next';
import { recreditOrder, refundOrder } from '../../api/payment';
import type { PaymentOrderListItem } from '../../api/payment';
import { useAuthStore } from '../../stores/useAuthStore';

const { Text } = Typography;

interface Props {
  type: 'recredit' | 'refund';
  order: PaymentOrderListItem | null;
  onClose: () => void;
  onSuccess: () => void;
}

export function RecreditRefundModal({ type, order, onClose, onSuccess }: Props) {
  const { t } = useTranslation();
  const role = useAuthStore((s) => s.admin?.role ?? '');
  const [reason, setReason] = useState('');
  const [confirmText, setConfirmText] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const isRecredit = type === 'recredit';
  const title = isRecredit
    ? t('payment.orders.detail.recredit')
    : t('payment.orders.detail.refund');

  const handleSubmit = async () => {
    if (!order) return;
    if (confirmText !== 'CONFIRM') return;
    if (!reason.trim()) {
      setError(t('payment.orders.detail.reasonRequired'));
      return;
    }

    setSubmitting(true);
    setError(null);
    try {
      if (isRecredit) {
        await recreditOrder(order.order_id, reason);
      } else {
        await refundOrder(order.order_id, reason);
      }
      onSuccess();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Unknown error');
    } finally {
      setSubmitting(false);
    }
  };

  const handleClose = () => {
    if (!submitting) {
      setReason('');
      setConfirmText('');
      setError(null);
      onClose();
    }
  };

  return (
    <Modal
      data-testid={`${type}-modal`}
      title={title}
      open={!!order}
      onCancel={handleClose}
      onOk={handleSubmit}
      okText={title}
      okButtonProps={{
        disabled: confirmText !== 'CONFIRM' || !reason.trim(),
        loading: submitting,
        danger: !isRecredit,
      }}
      destroyOnHidden
    >
      {order && role !== 'super_admin' ? (
        <Result
          status="403"
          title="403"
          subTitle="Only super_admin can perform this operation"
        />
      ) : order ? (
        <>
          <div style={{ marginBottom: 16 }}>
            <Text strong>Order: </Text>
            <Text code>{order.order_id.slice(0, 8)}...</Text>
            <br />
            <Text strong>State: </Text>
            <Text>{order.state}</Text>
          </div>

          {error && (
            <Alert type="error" message={error} style={{ marginBottom: 16 }} />
          )}

          <div style={{ marginBottom: 16 }}>
            <Text>{t('payment.orders.detail.reason')}:</Text>
            <Input.TextArea
              data-testid={`${type}-reason`}
              rows={3}
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              placeholder={t('payment.orders.detail.reasonRequired')}
            />
          </div>

          <div>
            <Text>
              {isRecredit
                ? t('payment.orders.detail.recreditConfirm')
                : t('payment.orders.detail.refundConfirm')}
            </Text>
            <Input
              data-testid={`${type}-confirm`}
              value={confirmText}
              onChange={(e) => setConfirmText(e.target.value)}
              placeholder="CONFIRM"
              style={{ marginTop: 8 }}
            />
          </div>
        </>
      ) : null}
    </Modal>
  );
}
