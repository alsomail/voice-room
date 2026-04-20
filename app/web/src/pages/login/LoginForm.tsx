/**
 * LoginForm — 管理员登录表单
 *
 * 职责（T-20001 TDS §二）：
 *   - Ant Design Form + Input + Button 构建登录表单
 *   - 账号输入框（Input）+ 密码输入框（Input.Password）
 *   - "记住密码" Checkbox，读写 localStorage['adminLoginUsername']
 *   - onSubmit 抛出异常时展示 Alert 错误提示
 *   - 全部文案通过 useTranslation() 提供，支持中英文切换
 */
import { useState } from 'react';
import { Form, Input, Button, Checkbox, Alert } from 'antd';
import { useTranslation } from 'react-i18next';
import styles from './login.module.css';

/** localStorage 中记住密码使用的 key（与测试保持一致） */
export const REMEMBER_KEY = 'adminLoginUsername';

export interface LoginFormValues {
  username: string;
  password: string;
  remember: boolean;
}

export interface LoginFormProps {
  /** 父组件注入的提交回调，抛出 Error 则展示错误提示 */
  onSubmit: (values: LoginFormValues) => Promise<void>;
}

export function LoginForm({ onSubmit }: LoginFormProps) {
  const { t } = useTranslation();
  const [form] = Form.useForm<LoginFormValues>();

  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // [MEDIUM-3] 懒初始化：只在组件挂载时读取一次 localStorage，避免每次渲染都读取
  const [initialValues] = useState<LoginFormValues>(() => {
    const saved = localStorage.getItem(REMEMBER_KEY);
    return { username: saved ?? '', password: '', remember: !!saved };
  });

  const handleFinish = async (values: LoginFormValues) => {
    setLoading(true);
    setError(null);

    try {
      await onSubmit(values);
      // 登录成功后处理"记住密码"
      if (values.remember) {
        localStorage.setItem(REMEMBER_KEY, values.username);
      } else {
        localStorage.removeItem(REMEMBER_KEY);
      }
    } catch (err) {
      const message =
        err instanceof Error ? err.message : t('login.error.unknown');
      setError(message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className={styles.formWrapper}>
      {error && (
        <Alert
          data-testid="alert-error"
          type="error"
          title={error}
          showIcon
          className={styles.errorAlert}
        />
      )}

      <Form
        form={form}
        initialValues={initialValues}
        onFinish={handleFinish}
        layout="vertical"
        size="large"
        autoComplete="off"
      >
        {/* 用户名 */}
        <Form.Item
          name="username"
          label={t('login.username')}
          rules={[
            {
              required: true,
              message: t('login.validation.usernameRequired'),
            },
          ]}
        >
          <Input
            data-testid="input-username"
            placeholder={t('login.usernamePlaceholder')}
            autoComplete="username"
          />
        </Form.Item>

        {/* 密码 */}
        <Form.Item
          name="password"
          label={t('login.password')}
          rules={[
            {
              required: true,
              message: t('login.validation.passwordRequired'),
            },
          ]}
        >
          <Input.Password
            data-testid="input-password"
            placeholder={t('login.passwordPlaceholder')}
            autoComplete="current-password"
          />
        </Form.Item>

        {/* 记住密码 */}
        <Form.Item name="remember" valuePropName="checked">
          <Checkbox>{t('login.rememberMe')}</Checkbox>
        </Form.Item>

        {/* 提交 */}
        <Form.Item>
          <Button
            data-testid="btn-submit"
            type="primary"
            htmlType="submit"
            loading={loading}
            disabled={loading}
            block
          >
            {t('login.submit')}
          </Button>
        </Form.Item>
      </Form>
    </div>
  );
}
