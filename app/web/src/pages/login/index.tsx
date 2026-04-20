/**
 * 管理员登录页
 *
 * 布局：全屏居中卡片，顶部品牌 Logo + 标题，底部 LoginForm
 * 数据流：LoginForm.onSubmit → useAuthStore.login → 成功跳转 /dashboard
 *
 * T-20002：已接入 useAuthStore.login，登录成功后跳转 /dashboard
 *          已认证用户直接重定向到 /dashboard
 */
import { Card, Typography } from 'antd';
import { useTranslation } from 'react-i18next';
import { Navigate, useNavigate } from 'react-router-dom';
import { LoginForm } from './LoginForm';
import type { LoginFormProps } from './LoginForm';
import { useAuthStore } from '../../stores/useAuthStore';
import styles from './login.module.css';

const { Title, Text } = Typography;

export function LoginPage() {
  const { t } = useTranslation();
  const login = useAuthStore((s) => s.login);
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  const navigate = useNavigate();

  // 已认证用户直接跳转到 /dashboard
  if (isAuthenticated) {
    return <Navigate to="/dashboard" replace />;
  }

  /**
   * 登录提交处理（T-20002：接入 useAuthStore.login）
   * 成功后跳转 /dashboard；失败时 LoginForm 内部展示 Alert
   */
  const handleSubmit: LoginFormProps['onSubmit'] = async (values) => {
    await login(values.username, values.password);
    navigate('/dashboard', { replace: true });
  };

  return (
    <div className={styles.pageContainer}>
      <Card className={styles.loginCard} variant="outlined">
        {/* 品牌区域 */}
        <div className={styles.brand}>
          {/* Logo 占位 —— T-20002 时替换为真实资源 */}
          <div className={styles.logoPlaceholder} aria-hidden="true">
            🎙️
          </div>
          <Title level={3} className={styles.title}>
            {t('login.title')}
          </Title>
          <Text type="secondary" className={styles.subtitle}>
            {t('login.subtitle')}
          </Text>
        </div>

        <LoginForm onSubmit={handleSubmit} />
      </Card>
    </div>
  );
}
