/**
 * GiftEditModal — 新增/编辑礼物弹窗（T-20012）
 *
 * Props：
 *   gift     — 编辑时传入现有礼物数据；null=新增
 *   open     — 弹窗是否显示
 *   onClose  — 关闭回调
 *   onSuccess — 保存成功回调
 *
 * 表单字段对齐 T-10014 API：
 *   code, name_en, name_ar, icon_url（Upload 或手动填写）
 *   price(≥1), tier([1-5]), effect_level([1-5])
 *   animation_url（可选），sort_order，is_active
 *
 * 图片上传：
 *   - Upload 组件，beforeUpload 校验 MIME（PNG/JPEG/WEBP）和大小（≤1MB）
 *   - 上传前调用 adminUploadGiftAsset(file, 'icon') 获取 URL 回填
 *   - 预览区实时显示图标
 *
 * 提交按钮禁用条件（useWatch 实时监听）：
 *   - price 为 0 或空
 */

import { useState, useEffect } from 'react';
import {
  Modal,
  Form,
  Input,
  InputNumber,
  Switch,
  Upload,
  Alert,
  Button,
  Space,
  Image,
  message,
} from 'antd';
import { UploadOutlined } from '@ant-design/icons';
import type { UploadFile, RcFile } from 'antd/es/upload';
import { useTranslation } from 'react-i18next';
import {
  adminCreateGift,
  adminUpdateGift,
  adminUploadGiftAsset,
  type AdminGiftItem,
  type AdminCreateGiftRequest,
} from '../../core/network/apiClient';

export interface GiftEditModalProps {
  gift: AdminGiftItem | null;
  open: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

// 允许的图片 MIME 类型
const ALLOWED_ICON_TYPES = ['image/png', 'image/jpeg', 'image/webp'];
const MAX_ICON_SIZE_BYTES = 1 * 1024 * 1024; // 1MB

export function GiftEditModal({ gift, open, onClose, onSuccess }: GiftEditModalProps) {
  const { t } = useTranslation();
  const [form] = Form.useForm<AdminCreateGiftRequest>();

  const [loading, setLoading] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [uploadError, setUploadError] = useState<string | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [fileList, setFileList] = useState<UploadFile[]>([]);

  // ── 实时监听 price 决定提交按钮禁用态
  const priceWatched = Form.useWatch('price', form) as number | undefined | null;
  const isSubmitDisabled = loading || !priceWatched || priceWatched <= 0;

  // ── 编辑时回填表单
  useEffect(() => {
    if (open && gift) {
      form.setFieldsValue({
        code: gift.code,
        name_en: gift.name_en,
        name_ar: gift.name_ar,
        icon_url: gift.icon_url,
        price: gift.price,
        tier: gift.tier,
        effect_level: gift.effect_level,
        animation_url: gift.animation_url ?? undefined,
        sort_order: gift.sort_order,
        is_active: gift.is_active,
      });
      setPreviewUrl(gift.icon_url);
    } else if (open && !gift) {
      form.resetFields();
      setPreviewUrl(null);
      setFileList([]);
    }
  }, [open, gift, form]);

  // ── 图标上传前校验
  const handleBeforeUpload = (file: RcFile): boolean => {
    setUploadError(null);

    if (!ALLOWED_ICON_TYPES.includes(file.type)) {
      setUploadError(t('gift.form.uploadTypeError'));
      return false;
    }
    if (file.size > MAX_ICON_SIZE_BYTES) {
      setUploadError(t('gift.form.uploadSizeError'));
      return false;
    }

    // 调用上传 API
    void (async () => {
      try {
        const result = await adminUploadGiftAsset(file, 'icon');
        form.setFieldValue('icon_url', result.url);
        setPreviewUrl(result.url);
        void message.success(result.file_name);
      } catch (err) {
        setUploadError(err instanceof Error ? err.message : t('common.requestError'));
      }
    })();

    // 不走 antd 内置上传，返回 false
    return false;
  };

  // ── 关闭重置
  const handleClose = () => {
    form.resetFields();
    setSaveError(null);
    setUploadError(null);
    setPreviewUrl(null);
    setFileList([]);
    onClose();
  };

  // ── 提交
  const handleSubmit = async () => {
    if (loading) return;
    let values: AdminCreateGiftRequest;
    try {
      values = await form.validateFields();
    } catch {
      return;
    }

    setLoading(true);
    setSaveError(null);
    try {
      if (gift) {
        await adminUpdateGift(gift.id, values);
      } else {
        await adminCreateGift(values);
      }
      onSuccess();
      handleClose();
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : t('common.requestError'));
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal
      data-testid="gift-edit-modal"
      open={open}
      title={gift ? t('gift.mgmt.editTitle') : t('gift.mgmt.createTitle')}
      onCancel={handleClose}
      footer={
        <Space>
          <Button onClick={handleClose}>{t('gift.form.cancelBtn')}</Button>
          <Button
            data-testid="gift-edit-submit-btn"
            type="primary"
            loading={loading}
            disabled={isSubmitDisabled}
            onClick={() => void handleSubmit()}
          >
            {t('gift.form.submitBtn')}
          </Button>
        </Space>
      }
      destroyOnHidden
      width={600}
    >
      {/* 保存失败提示 */}
      {saveError && (
        <Alert
          data-testid="gift-save-error"
          type="error"
          message={saveError}
          showIcon
          style={{ marginBottom: 16 }}
        />
      )}

      <Form form={form} layout="vertical">
        {/* 编码 */}
        <Form.Item
          name="code"
          label={t('gift.form.code')}
          rules={[
            { required: true, message: t('gift.form.codeRequired') },
            {
              pattern: /^[a-zA-Z0-9_]{1,32}$/,
              message: t('gift.form.codeFormat'),
            },
          ]}
        >
          <Input
            data-testid="gift-form-code"
            placeholder="unicorn_01"
            disabled={!!gift} // 编辑时不允许修改 code
          />
        </Form.Item>

        {/* 英文名称 */}
        <Form.Item
          name="name_en"
          label={t('gift.form.nameEn')}
          rules={[{ required: true, message: t('gift.form.nameEnRequired') }, { max: 64 }]}
        >
          <Input data-testid="gift-form-name-en" placeholder="Unicorn" />
        </Form.Item>

        {/* 阿拉伯文名称 */}
        <Form.Item
          name="name_ar"
          label={t('gift.form.nameAr')}
          rules={[{ required: true, message: t('gift.form.nameArRequired') }, { max: 64 }]}
        >
          <Input data-testid="gift-form-name-ar" placeholder="يونيكورن" />
        </Form.Item>

        {/* 图标上传 + 预览 */}
        <Form.Item label={t('gift.form.uploadIcon')}>
          {uploadError && (
            <Alert
              data-testid="upload-type-error"
              type="error"
              message={uploadError}
              showIcon
              style={{ marginBottom: 8 }}
            />
          )}
          <Space align="start">
            <Upload
              accept="image/png,image/jpeg,image/webp"
              beforeUpload={handleBeforeUpload}
              fileList={fileList}
              onChange={({ fileList: fl }) => setFileList(fl)}
              showUploadList={false}
            >
              <Button icon={<UploadOutlined />}>
                {t('gift.form.uploadIcon')}
              </Button>
              <input
                data-testid="gift-icon-upload-input"
                type="file"
                accept="image/png,image/jpeg,image/webp"
                style={{ display: 'none' }}
                onChange={(e) => {
                  const file = e.target.files?.[0];
                  if (file) {
                    handleBeforeUpload(file as RcFile);
                  }
                }}
              />
            </Upload>
            {previewUrl && (
              <Image
                data-testid="gift-icon-preview"
                src={previewUrl}
                width={64}
                height={64}
                style={{ objectFit: 'contain', border: '1px solid #d9d9d9', borderRadius: 4 }}
                preview={false}
                fallback="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
              />
            )}
          </Space>
        </Form.Item>

        {/* 图标 URL（手动填写） */}
        <Form.Item
          name="icon_url"
          label={t('gift.form.iconUrl')}
          rules={[{ required: true, message: t('gift.form.iconRequired') }]}
        >
          <Input
            data-testid="gift-form-icon-url"
            placeholder="/uploads/gifts/2026-04-21/unicorn.png"
            onChange={(e) => setPreviewUrl(e.target.value || null)}
          />
        </Form.Item>

        {/* 价格 */}
        <Form.Item
          name="price"
          label={t('gift.form.price')}
          rules={[
            { required: true, message: t('gift.form.priceRequired') },
            {
              validator: (_, value: number | null | undefined) => {
                if (value === null || value === undefined) return Promise.resolve();
                if (value < 1) return Promise.reject(new Error(t('gift.form.priceMin')));
                return Promise.resolve();
              },
            },
          ]}
        >
          <InputNumber
            data-testid="gift-form-price"
            min={1}
            style={{ width: '100%' }}
            placeholder="66"
          />
        </Form.Item>

        {/* Tier */}
        <Form.Item
          name="tier"
          label={t('gift.form.tier')}
          initialValue={1}
          rules={[{ required: true, message: t('gift.form.tierRequired') }]}
        >
          <InputNumber
            data-testid="gift-form-tier"
            min={1}
            max={5}
            style={{ width: '100%' }}
          />
        </Form.Item>

        {/* 特效级别 */}
        <Form.Item
          name="effect_level"
          label={t('gift.form.effectLevel')}
          initialValue={1}
          rules={[{ required: true, message: t('gift.form.effectLevelRequired') }]}
        >
          <InputNumber
            data-testid="gift-form-effect-level"
            min={1}
            max={5}
            style={{ width: '100%' }}
          />
        </Form.Item>

        {/* 动效 URL（可选） */}
        <Form.Item name="animation_url" label={t('gift.form.animationUrl')}>
          <Input
            data-testid="gift-form-animation-url"
            placeholder="https://cdn.example.com/unicorn.json"
          />
        </Form.Item>

        {/* 排序权重 */}
        <Form.Item name="sort_order" label={t('gift.form.sortOrder')} initialValue={0}>
          <InputNumber data-testid="gift-form-sort-order" min={0} style={{ width: '100%' }} />
        </Form.Item>

        {/* 是否上架 */}
        <Form.Item
          name="is_active"
          label={t('gift.form.isActive')}
          valuePropName="checked"
          initialValue={true}
        >
          <Switch data-testid="gift-form-is-active" />
        </Form.Item>
      </Form>
    </Modal>
  );
}
