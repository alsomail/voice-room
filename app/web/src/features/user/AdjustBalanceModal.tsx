/**
 * AdjustBalanceModal — 手动调整用户余额弹窗（T-20012）
 *
 * Props：
 *   userId         — 目标用户 ID
 *   currentBalance — 当前余额（展示用）
 *   open           — 弹窗是否显示
 *   onClose        — 关闭回调
 *   onSuccess      — 调整成功回调，传入新余额 newBalance
 *
 * 表单字段：
 *   amount  — InputNumber，可负数，必填非零，|amount| ≤ 10,000,000
 *   reason  — TextArea，2-200 字符，必填
 *
 * 逻辑：
 *   - 提交按钮：amount 为 0 或空 / reason 为空 时禁用（useWatch 实时监听）
 *   - amount < 0 时：Modal 内显示红色"扣减操作"警示 banner
 *   - amount < 0 提交时：Modal.confirm 二次确认
 *   - amount > 0 提交时：直接调用 API（无二次确认）
 *   - isConfirming 防并发重复提交
 *   - API 失败：显示 error Alert，弹窗保留
 *   - API 成功：onSuccess(newBalance) → onClose()
 */

import { useState } from 'react';
import {
  Modal,
  Form,
  InputNumber,
  Input,
  Button,
  Space,
  Alert,
  Typography,
} from 'antd';
import { WarningOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import {
  adminAdjustBalance,
  type AdminAdjustBalanceRequest,
} from '../../core/network/apiClient';

const { Text } = Typography;

export interface AdjustBalanceModalProps {
  userId: string;
  currentBalance: number;
  open: boolean;
  onClose: () => void;
  onSuccess: (newBalance: number) => void;
}

export function AdjustBalanceModal({
  userId,
  currentBalance,
  open,
  onClose,
  onSuccess,
}: AdjustBalanceModalProps) {
  const { t } = useTranslation();
  const [form] = Form.useForm<AdminAdjustBalanceRequest>();

  // ── 实时监听表单值（决定按钮禁用态）
  const amountWatched = Form.useWatch('amount', form) as number | undefined | null;
  const reasonWatched = Form.useWatch('reason', form) as string | undefined;

  const [loading, setLoading] = useState(false);
  const [isConfirming, setIsConfirming] = useState(false);
  const [adjustError, setAdjustError] = useState<string | null>(null);

  // ── 是否是扣减操作
  const isDeduct = typeof amountWatched === 'number' && amountWatched < 0;

  // ── 提交按钮禁用态
  const isSubmitDisabled =
    loading ||
    isConfirming ||
    !amountWatched ||
    amountWatched === 0 ||
    !reasonWatched?.trim();

  // ── 实际调用 API
  const doAdjust = async (values: AdminAdjustBalanceRequest) => {
    setLoading(true);
    setAdjustError(null);
    try {
      const result = await adminAdjustBalance(userId, values);
      onSuccess(result.new_balance);
      onClose();
    } catch (err) {
      setAdjustError(
        err instanceof Error ? err.message : t('common.requestError'),
      );
    } finally {
      setLoading(false);
    }
  };

  // ── 提交：校验 → 负数二次确认 → API
  const handleSubmit = async () => {
    if (loading || isConfirming) return;

    let values: AdminAdjustBalanceRequest;
    try {
      values = await form.validateFields();
    } catch {
      return;
    }

    if (values.amount < 0) {
      // 负数：二次 Modal.confirm
      setIsConfirming(true);
      Modal.confirm({
        title: t('wallet.adjust.confirmTitle'),
        content: t('wallet.adjust.confirmDeduct', { amount: Math.abs(values.amount) }),
        okText: t('wallet.adjust.submitBtn'),
        cancelText: t('wallet.adjust.cancelBtn'),
        okButtonProps: { danger: true },
        afterClose: () => setIsConfirming(false),
        onOk: async () => {
          await doAdjust(values);
        },
      });
    } else {
      // 正数：直接调用
      await doAdjust(values);
    }
  };

  // ── 关闭时重置
  const handleClose = () => {
    form.resetFields();
    setAdjustError(null);
    setIsConfirming(false);
    onClose();
  };

  return (
    <Modal
      open={open}
      title={t('wallet.adjust.title')}
      onCancel={handleClose}
      footer={
        <Space>
          <Button onClick={handleClose}>{t('wallet.adjust.cancelBtn')}</Button>
          <Button
            data-testid="adjust-submit-btn"
            type="primary"
            loading={loading || isConfirming}
            disabled={isSubmitDisabled}
            onClick={() => void handleSubmit()}
          >
            {t('wallet.adjust.submitBtn')}
          </Button>
        </Space>
      }
      destroyOnHidden
    >
      {/* 扣减操作红色警示 */}
      {isDeduct && (
        <Alert
          data-testid="deduct-warning-banner"
          type="warning"
          icon={<WarningOutlined />}
          message={
            <Text strong style={{ color: '#d4380d' }}>
              {t('wallet.adjust.deductWarning')}
            </Text>
          }
          showIcon
          style={{ marginBottom: 16, borderColor: '#d4380d', background: '#fff2e8' }}
        />
      )}

      {/* 调整失败错误提示 */}
      {adjustError && (
        <Alert
          data-testid="adjust-error-alert"
          type="error"
          message={adjustError}
          showIcon
          style={{ marginBottom: 16 }}
        />
      )}

      {/* 当前余额展示 */}
      <div style={{ marginBottom: 16, color: '#888' }}>
        {t('users.drawer.coinBalance')}: {currentBalance}
      </div>

      <Form form={form} layout="vertical">
        {/* 金额 */}
        <Form.Item
          name="amount"
          label={t('wallet.adjust.amount')}
          rules={[
            { required: true, message: t('wallet.adjust.amountRequired') },
            {
              validator: (_, value: number | null | undefined) => {
                if (value === null || value === undefined) return Promise.resolve();
                if (value === 0) return Promise.reject(new Error(t('wallet.adjust.amountNonZero')));
                if (Math.abs(value) > 10_000_000)
                  return Promise.reject(new Error(t('wallet.adjust.amountMax')));
                return Promise.resolve();
              },
            },
          ]}
        >
          <InputNumber
            data-testid="adjust-amount-input"
            style={{ width: '100%' }}
            placeholder="-1000 / 500"
          />
        </Form.Item>

        {/* 原因 */}
        <Form.Item
          name="reason"
          label={t('wallet.adjust.reason')}
          rules={[
            { required: true, message: t('wallet.adjust.reasonRequired') },
            { min: 2, message: t('wallet.adjust.reasonMin') },
            { max: 200, message: t('wallet.adjust.reasonMax') },
          ]}
        >
          <Input.TextArea
            data-testid="adjust-reason-input"
            placeholder={t('wallet.adjust.reason')}
            rows={3}
            maxLength={200}
            showCount
          />
        </Form.Item>
      </Form>
    </Modal>
  );
}
