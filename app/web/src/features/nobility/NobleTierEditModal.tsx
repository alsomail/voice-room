/**
 * NobleTierEditModal — 贵族等级创建/编辑弹窗 (T-20035)
 */

import { useState, useEffect } from 'react';
import { Modal, Form, Input, InputNumber, Alert } from 'antd';
import { useTranslation } from 'react-i18next';
import {
  createNobleTier,
  updateNobleTier,
  type TierItem,
  type CreateTierRequest,
} from '../../api/nobility';

interface Props {
  open: boolean;
  tier: TierItem | null; // null = create mode
  onClose: () => void;
  onSuccess: () => void;
}

export function NobleTierEditModal({ open, tier, onClose, onSuccess }: Props) {
  const { t } = useTranslation();
  const [form] = Form.useForm();
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const isCreate = !tier;

  useEffect(() => {
    if (open) {
      if (tier) {
        form.setFieldsValue(tier);
      } else {
        form.resetFields();
      }
      setError(null);
    }
  }, [open, tier, form]);

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();
      setSubmitting(true);
      setError(null);

      if (isCreate) {
        await createNobleTier({
          ...values,
          privileges: typeof values.privileges === 'string'
            ? JSON.parse(values.privileges)
            : values.privileges ?? {},
        } as CreateTierRequest);
      } else {
        const data: Record<string, unknown> = {};
        for (const [k, v] of Object.entries(values)) {
          if (v !== tier![k as keyof TierItem]) data[k] = v;
        }
        await updateNobleTier(tier!.tier_id, data as never);
      }

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
      data-testid="noble-tier-edit-modal"
      title={isCreate ? t('nobility.tiers.add') : t('nobility.tiers.edit')}
      open={open}
      onCancel={onClose}
      onOk={handleSubmit}
      okText={t('nobility.tiers.form.submitBtn')}
      cancelText={t('nobility.tiers.form.cancelBtn')}
      confirmLoading={submitting}
      destroyOnHidden
      width={600}
    >
      {error && <Alert type="error" message={error} style={{ marginBottom: 16 }} />}

      <Form form={form} layout="vertical">
        <Form.Item
          name="tier_id"
          label={t('nobility.tiers.form.tierId')}
          rules={[{ required: true, message: t('nobility.tiers.form.tierIdRequired') }]}
        >
          <Input disabled={!isCreate} />
        </Form.Item>

        <Form.Item
          name="name_en"
          label={t('nobility.tiers.form.nameEn')}
          rules={[{ required: true, message: t('nobility.tiers.form.nameEnRequired') }]}
        >
          <Input />
        </Form.Item>

        <Form.Item
          name="name_ar"
          label={t('nobility.tiers.form.nameAr')}
          rules={[{ required: true, message: t('nobility.tiers.form.nameArRequired') }]}
        >
          <Input />
        </Form.Item>

        <Form.Item
          name="level"
          label={t('nobility.tiers.form.level')}
          rules={[
            { required: true, message: t('nobility.tiers.form.levelRequired') },
            { type: 'number', min: 1, max: 6, message: t('nobility.tiers.form.levelRange') },
          ]}
        >
          <InputNumber style={{ width: '100%' }} min={1} max={6} />
        </Form.Item>

        <Form.Item
          name="monthly_diamonds"
          label={t('nobility.tiers.form.monthlyDiamonds')}
          rules={[
            { required: true, message: t('nobility.tiers.form.monthlyDiamondsRequired') },
            { type: 'number', min: 1, message: t('nobility.tiers.form.monthlyDiamondsMin') },
          ]}
        >
          <InputNumber style={{ width: '100%' }} min={1} />
        </Form.Item>

        <Form.Item
          name="monthly_usd"
          label={t('nobility.tiers.form.monthlyUsd')}
          rules={[{ required: true, message: t('nobility.tiers.form.monthlyUsdRequired') }]}
        >
          <Input placeholder="9.99" />
        </Form.Item>

        <Form.Item name="usd_sku_id" label={t('nobility.tiers.form.usdSkuId')}>
          <Input />
        </Form.Item>

        <Form.Item
          name="icon_url"
          label={t('nobility.tiers.form.iconUrl')}
          rules={[{ required: true, message: t('nobility.tiers.form.iconRequired') }]}
        >
          <Input />
        </Form.Item>

        <Form.Item
          name="frame_url"
          label={t('nobility.tiers.form.frameUrl')}
          rules={[{ required: true, message: t('nobility.tiers.form.frameRequired') }]}
        >
          <Input />
        </Form.Item>

        <Form.Item name="entrance_animation_url" label={t('nobility.tiers.form.entranceAnimationUrl')}>
          <Input />
        </Form.Item>

        <Form.Item name="bgm_url" label={t('nobility.tiers.form.bgmUrl')}>
          <Input />
        </Form.Item>

        <Form.Item
          name="badge_color"
          label={t('nobility.tiers.form.badgeColor')}
          rules={[{ required: true, message: t('nobility.tiers.form.badgeColorRequired') }]}
        >
          <Input />
        </Form.Item>

        <Form.Item
          name="bubble_style_id"
          label={t('nobility.tiers.form.bubbleStyleId')}
          rules={[{ required: true, message: t('nobility.tiers.form.bubbleStyleRequired') }]}
        >
          <Input />
        </Form.Item>

        <Form.Item name="privileges" label={t('nobility.tiers.form.privileges')}>
          <Input.TextArea rows={4} placeholder='{"invisible": true, "gift_discount": 0.1}' />
        </Form.Item>
      </Form>
    </Modal>
  );
}
