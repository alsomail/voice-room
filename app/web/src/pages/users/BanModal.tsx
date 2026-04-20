/**
 * BanModal — 封禁用户对话框（T-20008）
 *
 * Props：
 *   userId   — 当前要封禁的用户 ID（null 时 Modal 关闭）
 *   onClose  — Modal 取消/关闭回调
 *   onSuccess — 封禁成功回调（传入 userId）
 *
 * 表单字段：
 *   duration — 封禁时长（Select，必填）
 *   reason   — 封禁原因（Select，必填）
 *   remark   — 备注（TextArea，可选，最多 200 字）
 *
 * 提交流程：
 *   validateFields → Modal.confirm 二次确认 → useBanUser.ban → onSuccess
 *   失败时显示 error Alert，不关闭 Modal
 */

import { useState } from 'react';
import { Modal, Form, Select, Input, Button, Space, Alert } from 'antd';
import { useTranslation } from 'react-i18next';
import { useBanUser } from './useBanUser';

export interface BanModalProps {
  userId: string | null;
  onClose: () => void;
  onSuccess: (userId: string) => void;
}

export function BanModal({ userId, onClose, onSuccess }: BanModalProps) {
  const { t } = useTranslation();
  const [form] = Form.useForm();
  const { loading, ban } = useBanUser();
  const [banError, setBanError] = useState<string | null>(null);
  // 防并发：Modal.confirm 弹出期间（用户尚未点 OK/Cancel）禁用提交按钮
  const [isConfirming, setIsConfirming] = useState(false);

  // ── 封禁时长选项 ──────────────────────────────────────────────────────────
  const durationOptions = [
    { label: t('users.ban.duration24h'), value: 1440 },
    { label: t('users.ban.duration72h'), value: 4320 },
    { label: t('users.ban.duration7d'), value: 10080 },
    { label: t('users.ban.duration30d'), value: 43200 },
    { label: t('users.ban.durationForever'), value: null },
  ];

  // ── 封禁原因选项 ──────────────────────────────────────────────────────────
  const reasonOptions = [
    { label: t('users.ban.reasonViolation'), value: '违规内容' },
    { label: t('users.ban.reasonHarass'), value: '骚扰他人' },
    { label: t('users.ban.reasonFraud'), value: '刷单行为' },
    { label: t('users.ban.reasonOther'), value: '其他' },
  ];

  // ── 提交：先校验，再二次确认，再调用 ban ──────────────────────────────────
  const handleSubmit = async () => {
    if (loading || isConfirming) return;
    let values: { duration: number | null; reason: string; remark?: string };
    try {
      values = await form.validateFields();
    } catch {
      // 校验失败，antd Form 已自动显示错误信息
      return;
    }

    setIsConfirming(true);
    Modal.confirm({
      title: t('users.ban.confirmTitle'),
      content: t('users.ban.confirmContent'),
      okText: t('users.ban.submitBtn'),
      cancelText: t('users.ban.cancelBtn'),
      afterClose: () => setIsConfirming(false),
      onOk: async () => {
        try {
          setBanError(null);
          await ban(userId!, {
            action: 'ban',
            duration: values.duration,
            reason: values.reason,
            remark: values.remark ?? '',
          });
          onSuccess(userId!);
        } catch (err) {
          const errMsg =
            err instanceof Error && err.message.includes('40901')
              ? t('users.ban.alreadyBanned')
              : err instanceof Error
                ? err.message
                : t('common.requestError');
          setBanError(errMsg);
          // 不 re-throw：confirm dialog 自动关闭，BanModal 保持显示（含 error Alert）
        }
      },
    });
  };

  // ── 关闭时重置表单 ────────────────────────────────────────────────────────
  const handleClose = () => {
    form.resetFields();
    setBanError(null);
    setIsConfirming(false);  // 修复：防止外层 Modal 先关闭导致 state 残留
    onClose();
  };

  return (
    <Modal
      open={!!userId}
      title={t('users.ban.title')}
      onCancel={handleClose}
      footer={
        <Space>
          <Button onClick={handleClose}>{t('users.ban.cancelBtn')}</Button>
          <Button
            type="primary"
            danger
            loading={loading || isConfirming}
            disabled={loading || isConfirming}
            onClick={() => void handleSubmit()}
          >
            {t('users.ban.submitBtn')}
          </Button>
        </Space>
      }
      destroyOnHidden
    >
      {/* 封禁失败错误提示 */}
      {banError && (
        <Alert
          data-testid="ban-error-alert"
          type="error"
          message={banError}
          showIcon
          style={{ marginBottom: 16 }}
        />
      )}

      <Form form={form} layout="vertical">
        {/* 封禁时长 */}
        <Form.Item
          name="duration"
          label={t('users.ban.durationLabel')}
          rules={[{ required: true, message: t('users.ban.durationRequired') }]}
        >
          <Select
            data-testid="ban-duration-select"
            placeholder={t('users.ban.durationLabel')}
            options={durationOptions}
          />
        </Form.Item>

        {/* 封禁原因 */}
        <Form.Item
          name="reason"
          label={t('users.ban.reasonLabel')}
          rules={[{ required: true, message: t('users.ban.reasonRequired') }]}
        >
          <Select
            data-testid="ban-reason-select"
            placeholder={t('users.ban.reasonLabel')}
            options={reasonOptions}
          />
        </Form.Item>

        {/* 备注 */}
        <Form.Item
          name="remark"
          label={t('users.ban.remarkLabel')}
          rules={[
            {
              max: 200,
              message: t('users.ban.remarkMaxLength'),
            },
          ]}
        >
          <Input.TextArea
            data-testid="ban-remark-textarea"
            placeholder={t('users.ban.remarkPlaceholder')}
            maxLength={200}
            rows={3}
          />
        </Form.Item>
      </Form>
    </Modal>
  );
}
