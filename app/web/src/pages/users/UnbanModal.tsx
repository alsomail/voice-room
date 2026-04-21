/**
 * UnbanModal — 解封用户对话框（T-20010）
 *
 * Props：
 *   userId   — 当前要解封的用户 ID（null 时 Modal 关闭）
 *   onClose  — Modal 取消/关闭回调
 *   onSuccess — 解封成功回调（传入 userId）
 *
 * 表单字段：
 *   reason — 解封原因（Select，必填）
 *   remark — 备注（TextArea，可选，最多 200 字）
 *
 * 提交流程：
 *   validateFields → Modal.confirm 二次确认 → useUnbanUser.unban → onSuccess
 *   失败时显示 error Alert，不关闭 Modal
 */

import { useState } from 'react';
import { Modal, Form, Select, Input, Button, Space, Alert } from 'antd';
import { useTranslation } from 'react-i18next';
import { useUnbanUser } from './useUnbanUser';

export interface UnbanModalProps {
  userId: string | null;
  onClose: () => void;
  onSuccess: (userId: string) => void;
}

export function UnbanModal({ userId, onClose, onSuccess }: UnbanModalProps) {
  const { t } = useTranslation();
  const [form] = Form.useForm();
  const { loading, unban } = useUnbanUser();
  const [unbanError, setUnbanError] = useState<string | null>(null);
  // 防并发：Modal.confirm 弹出期间（用户尚未点 OK/Cancel）禁用提交按钮
  const [isConfirming, setIsConfirming] = useState(false);

  // ── 解封原因选项 ──────────────────────────────────────────────────────────
  const reasonOptions = [
    { label: t('users.unban.reasonExpired'), value: '处罚到期' },
    { label: t('users.unban.reasonMistake'), value: '误封' },
    { label: t('users.unban.reasonAppeal'), value: '申诉核实' },
    { label: t('users.unban.reasonOther'), value: '其他' },
  ];

  // ── 提交：先校验，再二次确认，再调用 unban ────────────────────────────────
  const handleSubmit = async () => {
    if (loading || isConfirming) return;
    let values: { reason: string; remark?: string };
    try {
      values = await form.validateFields();
    } catch {
      return;
    }

    setIsConfirming(true);
    Modal.confirm({
      title: t('users.unban.confirmTitle'),
      content: t('users.unban.confirmContent'),
      okText: t('users.unban.submitBtn'),
      cancelText: t('users.unban.cancelBtn'),
      afterClose: () => setIsConfirming(false),
      onOk: async () => {
        try {
          setUnbanError(null);
          await unban(userId!, {
            reason: values.reason,
            remark: values.remark,
          });
          onSuccess(userId!);
        } catch (err) {
          const errMsg =
            err instanceof Error && err.message.includes('40901')
              ? t('users.unban.alreadyNormal')
              : err instanceof Error
                ? err.message
                : t('common.requestError');
          setUnbanError(errMsg);
          // 不 re-throw：confirm dialog 自动关闭，UnbanModal 保持显示（含 error Alert）
        }
      },
    });
  };

  // ── 关闭时重置表单 ────────────────────────────────────────────────────────
  const handleClose = () => {
    form.resetFields();
    setUnbanError(null);
    setIsConfirming(false); // 防止外层 Modal 先关闭导致 state 残留
    onClose();
  };

  return (
    <Modal
      open={!!userId}
      title={t('users.unban.modalTitle')}
      onCancel={handleClose}
      footer={
        <Space>
          <Button
            data-testid="unban-cancel-btn"
            onClick={handleClose}
          >
            {t('users.unban.cancelBtn')}
          </Button>
          <Button
            data-testid="unban-confirm-btn"
            type="primary"
            loading={loading || isConfirming}
            disabled={loading || isConfirming}
            onClick={() => void handleSubmit()}
          >
            {t('users.unban.submitBtn')}
          </Button>
        </Space>
      }
      destroyOnHidden
    >
      <div data-testid="unban-modal">
        {/* 解封失败错误提示 */}
        {unbanError && (
          <Alert
            data-testid="unban-error-alert"
            type="error"
            message={unbanError}
            showIcon
            style={{ marginBottom: 16 }}
          />
        )}

        <Form form={form} layout="vertical">
          {/* 解封原因 */}
          <Form.Item
            name="reason"
            label={t('users.unban.reasonLabel')}
            rules={[{ required: true, message: t('users.unban.reasonRequired') }]}
          >
            <Select
              data-testid="unban-reason-select"
              placeholder={t('users.unban.reasonLabel')}
              options={reasonOptions}
            />
          </Form.Item>

          {/* 备注 */}
          <Form.Item
            name="remark"
            label={t('users.unban.remarkLabel')}
            rules={[
              {
                max: 200,
                message: t('users.unban.remarkMaxLen'),
              },
            ]}
          >
            <Input.TextArea
              data-testid="unban-remark-input"
              placeholder={t('users.unban.remarkLabel')}
              maxLength={200}
              rows={3}
            />
          </Form.Item>
        </Form>
      </div>
    </Modal>
  );
}
