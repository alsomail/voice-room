/**
 * SkuEditModal — SKU 创建/编辑弹窗 (T-20032)
 */

import { useState, useEffect } from 'react';
import { Modal, Form, Input, InputNumber, Switch, Alert, Typography } from 'antd';
import { useTranslation } from 'react-i18next';
import { createSku, updateSku, type SkuItem, type SkuCreateRequest } from '../../api/payment';

const { Text } = Typography;

interface Props {
  open: boolean;
  sku: SkuItem | null; // null = create mode
  onClose: () => void;
  onSuccess: () => void;
}

export function SkuEditModal({ open, sku, onClose, onSuccess }: Props) {
  const { t } = useTranslation();
  const [form] = Form.useForm();
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [warning, setWarning] = useState<string | null>(null);

  const isCreate = !sku;

  useEffect(() => {
    if (open) {
      if (sku) {
        form.setFieldsValue(sku);
      } else {
        form.resetFields();
      }
      setError(null);
      setWarning(null);
    }
  }, [open, sku, form]);

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();
      setSubmitting(true);
      setError(null);
      setWarning(null);

      if (isCreate) {
        const result = await createSku(values as SkuCreateRequest);
        if (result.warning) setWarning(result.warning);
      } else {
        // Check price change confirm
        if (values.diamonds !== sku?.diamonds || values.display_price_usd !== sku?.display_price_usd) {
          const confirmed = window.confirm(t('payment.skus.form.priceChangeConfirm'));
          if (!confirmed) { setSubmitting(false); return; }
        }
        await updateSku(sku!.sku_id, values, true);
      }

      onSuccess();
    } catch (e: unknown) {
      if (e && typeof e === 'object' && 'errorFields' in e) return; // form validation
      setError(e instanceof Error ? e.message : 'Unknown error');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Modal
      data-testid="sku-edit-modal"
      title={isCreate ? t('payment.skus.add') : t('payment.skus.edit')}
      open={open}
      onCancel={onClose}
      onOk={handleSubmit}
      okText={t('payment.skus.form.submitBtn')}
      cancelText={t('payment.skus.form.cancelBtn')}
      confirmLoading={submitting}
      destroyOnHidden
      width={520}
    >
      {warning && <Alert type="warning" message={warning} style={{ marginBottom: 16 }} />}
      {error && <Alert type="error" message={error} style={{ marginBottom: 16 }} />}

      <Form form={form} layout="vertical">
        <Form.Item
          name="sku_id"
          label={t('payment.skus.form.skuId')}
          rules={[{ required: true, message: t('payment.skus.form.skuIdRequired') }]}
          extra={t('payment.skus.form.skuIdHint')}
        >
          <Input disabled={!isCreate} />
        </Form.Item>

        <Form.Item
          name="provider"
          label={t('payment.skus.form.provider')}
          initialValue="google_play"
          rules={[{ required: true, message: t('payment.skus.form.providerRequired') }]}
        >
          <Input disabled={!isCreate} />
        </Form.Item>

        <Form.Item
          name="diamonds"
          label={t('payment.skus.form.diamonds')}
          rules={[
            { required: true, message: t('payment.skus.form.diamondsRequired') },
            { type: 'number', min: 1, message: t('payment.skus.form.diamondsMin') },
          ]}
        >
          <InputNumber style={{ width: '100%' }} min={1} />
        </Form.Item>

        <Form.Item
          name="display_price_usd"
          label={t('payment.skus.form.priceUsd')}
          rules={[
            { required: true, message: t('payment.skus.form.priceRequired') },
          ]}
        >
          <Input placeholder="9.99" />
        </Form.Item>

        <Form.Item name="sort_order" label={t('payment.skus.form.sortOrder')}>
          <InputNumber style={{ width: '100%' }} min={0} />
        </Form.Item>

        <Form.Item name="tag" label={t('payment.skus.form.tag')} extra={t('payment.skus.form.tagHint')}>
          <Input />
        </Form.Item>

        <Form.Item name="is_active" label={t('payment.skus.form.isActive')} valuePropName="checked" initialValue={true}>
          <Switch />
        </Form.Item>
      </Form>
    </Modal>
  );
}
