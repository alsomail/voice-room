/**
 * AppLayout — 管理后台侧栏布局（T-20012）
 *
 * 功能：
 *   - Ant Design Layout + Sider + Menu 标准后台布局
 *   - 侧栏菜单项根据管理员角色 RBAC 控制可见性
 *   - 礼物管理菜单（/gifts）仅 super_admin / operator 可见（W12-07）
 *   - Outlet 渲染子路由内容
 *
 * 权限矩阵（T-10014 RBAC）：
 *   | 角色         | 礼物管理菜单 |
 *   |-------------|------------|
 *   | super_admin | ✅          |
 *   | operator    | ✅          |
 *   | cs          | ❌          |
 *   | finance     | ❌          |
 */

import { useState } from 'react';
import { Layout, Menu } from 'antd';
import {
  DashboardOutlined,
  TeamOutlined,
  HomeOutlined,
  FileTextOutlined,
  GiftOutlined,
} from '@ant-design/icons';
import { useNavigate, useLocation, Outlet } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useAuthStore } from '../stores/useAuthStore';

const { Sider, Content } = Layout;

// 有权访问礼物管理菜单的角色
const GIFT_MENU_ROLES = ['super_admin', 'operator'];

export function AppLayout() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();
  const [collapsed, setCollapsed] = useState(false);

  const admin = useAuthStore((s) => s.admin);
  const role = admin?.role ?? '';

  const canSeeGiftMenu = GIFT_MENU_ROLES.includes(role);

  // ── 菜单项
  const menuItems = [
    {
      key: '/dashboard',
      icon: <DashboardOutlined />,
      label: t('dashboard.title'),
      'data-testid': 'menu-item-dashboard',
    },
    {
      key: '/rooms',
      icon: <HomeOutlined />,
      label: t('rooms.title'),
      'data-testid': 'menu-item-rooms',
    },
    {
      key: '/users',
      icon: <TeamOutlined />,
      label: t('users.title'),
      'data-testid': 'menu-item-users',
    },
    {
      key: '/logs',
      icon: <FileTextOutlined />,
      label: t('logs.title'),
      'data-testid': 'menu-item-logs',
    },
    // ── 礼物管理：仅 super_admin / operator 可见（W12-07）
    ...(canSeeGiftMenu
      ? [
          {
            key: '/gifts',
            icon: <GiftOutlined />,
            label: t('gift.mgmt.title'),
            'data-testid': 'menu-item-gifts',
          },
        ]
      : []),
  ];

  // ── 当前选中菜单 key（根据路由路径）
  const selectedKey = menuItems.find((item) =>
    location.pathname.startsWith(item.key),
  )?.key ?? '/dashboard';

  return (
    <Layout style={{ minHeight: '100vh' }}>
      <Sider
        collapsible
        collapsed={collapsed}
        onCollapse={setCollapsed}
        theme="dark"
        data-testid="app-sider"
      >
        {/* Logo 区域 */}
        <div
          style={{
            height: 32,
            margin: 16,
            background: 'rgba(255,255,255,0.2)',
            borderRadius: 4,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: '#fff',
            fontSize: 12,
          }}
        >
          {collapsed ? 'VR' : 'Voice Room Admin'}
        </div>

        <Menu
          theme="dark"
          mode="inline"
          selectedKeys={[selectedKey]}
          onClick={({ key }) => void navigate(key)}
          items={menuItems.map(({ 'data-testid': testId, ...rest }) => ({
            ...rest,
            // 通过 label 包裹 span 实现 data-testid（Menu.Item 不直接支持 data-testid）
            label: (
              <span data-testid={testId}>
                {rest.label}
              </span>
            ),
          }))}
        />
      </Sider>

      <Layout>
        <Content style={{ padding: 0, overflow: 'initial' }}>
          <Outlet />
        </Content>
      </Layout>
    </Layout>
  );
}
