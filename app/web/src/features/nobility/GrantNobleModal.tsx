/**
 * GrantNobleModal — 赠送/撤销贵族弹窗 (T-20036)
 *
 * super_admin only。用于从用户详情 Drawer 的贵族 Tab 触发。
 */

import { useState, useEffect } from 'react';
import { Modal, Form, Input, InputNumber, Select, Alert, Typography } from 'antd';
import { useTranslation } from 'react-i18next';
import { grantNoble, revokeNoble, listNobleTiers, type TierItem } from '../../api/nobility';

const { Text } = Typography;

interface GrantProps {
  open: boolean;
  userId: string;
  onClose: () => void;
  onSuccess: () => void;
}

export function GrantNobleModal({ open, userId, onClose, onSuccess }: GrantProps) {
  const { t } = useTranslation();
  const [form] = Form.useForm();
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tiers, setTiers] = useState<TierItem[]>([]);

  useEffect(() => {
    if (open) {
      form.resetFields();
      setError(null);
      listNobleTiers(1, 50)
        .then((r) => setTiers(r.items.filter((t) => t.is_active)))
        .catch(() => setTiers([]));
    }
  }, [open, form]);

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();
      setSubmitting(true);
      setError(null);
      await grantNoble(userId, values.tier_id, values.duration_days, values.reason);
      onSuccess();
    } catch (e: unknown) {
      if (e && typeof e === 'object' && 'errorFields' in e) return;
      setError(e instanceof Error ? e.message : 'Unknown error');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Modal
      data-testid="grant-noble-modal"
      title={t('nobility.users.grant.title')}
      open={open}
      onCancel={onClose}
      onOk={handleSubmit}
      okText={t('nobility.users.grant.title')}
      cancelText={t('nobility.tiers.form.cancelBtn')}
      confirmLoading={submitting}
      destroyOnHidden
      width={440}
    >
      {error && <Alert type="error" message={error} style={{ marginBottom: 16 }} />}

      <Form form={form} layout="vertical">
        <Form.Item
          name="tier_id"
          label={t('nobility.users.grant.tierId')}
          rules={[{ required: true, message: t('nobility.users.grant.tierRequired') }]}
        >
          <Select
            options={tiers.map((t) => ({
              value: t.tier_id,
              label: `${t.name_en} (Lv.${t.level})`,
            }))}
          />
        </Form.Item>

        <Form.Item
          name="duration_days"
          label={t('nobility.users.grant.durationDays')}
          rules={[
            { required: true, message: t('nobility.users.grant.durationRequired') },
            { type: 'number', min: 1, max: 365, message: t('nobility.users.grant.durationRange') },
          ]}
          initialValue={30}
        >
          <InputNumber style={{ width: '100%' }} min={1} max={365} />
        </Form.Item>

        <Form.Item
          name="reason"
          label={t('nobility.users.grant.reason')}
          rules={[{ required: true, message: t('nobility.users.grant.reasonRequired') }]}
        >
          <Input.TextArea rows={3} />
        </Form.Item>
      </Form>
    </Modal>
  );
}

interface RevokeProps {
  open: boolean;
  userId: string;
  onClose: () => void;
  onSuccess: () => void;
}

export function RevokeNobleModal({ open, userId, onClose, onSuccess }: RevokeProps) {
  const { t } = useTranslation();
  const [form] = Form.useForm();
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      form.resetFields();
      setError(null);
    }
  }, [open, form]);

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();
      const confirmed = window.confirm(t('nobility.users.revoke.confirm'));
      if (!confirmed) return;
      setSubmitting(true);
      setError(null);
      await revokeNoble(userId, values.reason);
      onSuccess();
    } catch (e: unknown) {
      if (e && typeof e === 'object' && 'errorFields' in e) return;
      setError(e instanceof Error ? e.message : 'Unknown error');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Modal
      data-testid="revoke-noble-modal"
      title={t('nobility.users.revoke.title')}
      open={open}
      onCancel={onClose}
      onOk={handleSubmit}
      okText={t('nobility.users.revoke.button')}
      okButtonProps={{ danger: true }}
      cancelText={t('nobility.tiers.form.cancelBtn')}
      confirmLoading={submitting}
      destroyOnHidden
      width={440}
    >
      {error && <Alert type="error" message={error} style={{ marginBottom: 16 }} />}

      <Form form={form} layout="vertical">
        <Form.Item
          name="reason"
          label={t('nobility.users.revoke.reason')}
          rules={[{ required: true, message: t('nobility.users.revoke.reasonRequired') }]}
        >
          <Input.TextArea rows={3} />
        </Form.Item>
      </Form>
    </Modal>
  );
}
