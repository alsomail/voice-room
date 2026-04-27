# 测试套件：AUTH 管理员登录（Web）

> **需求模糊点 (Ambiguity Notes)**：
> - 无

覆盖 Task：T-20001（Admin 登录页）、T-20002（路由守卫）、T-20003（i18n）。

---

## TC-AUTH-00001：Admin 登录页 UI + 记住用户名
**【元数据】**
- **归属模块**：`AUTH`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. 全局前置见 [_README.md §一](../_README.md#一所有用例默认前置条件隐式前置)；Playwright `use.baseURL` 已由 envLoader 注入为 `${ADMIN_WEB_URL}`，用例可直接 `page.goto('/login')`（相对路径）。
2. 浏览器 localStorage 清空。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 页面加载 | 居中卡片，含顶部"语聊房管理后台"Logo、用户名输入框、密码输入框、"记住账号"Checkbox、蓝色"登录"按钮 |
| 2 | `AdminWeb` | 在用户名框输入 `admin_op` | 框内文字更新 |
| 3 | `AdminWeb` | 在密码框输入 `Pass@123` | 密码以 `•` 呈现 |
| 4 | `AdminWeb` | 勾选"记住账号" 后点击"登录" | 登录按钮出现 loading spinner |
| 5 | `AdminWeb` | 成功后 | 跳转至 `/dashboard`；URL 变化为 `/dashboard` |
| 6 | `AdminWeb` | 退出登录后回到 /login | 用户名框自动填入 `admin_op`，密码框为空，"记住账号"仍勾选 |

**【数据清理】**
- 清空 localStorage。

---

## TC-AUTH-00002：登录失败 - 错误凭证
**【元数据】**
- **归属模块**：`AUTH`
- **测试类型**：`Functional`
- **回归级别**：`P1`

**【前置条件】**
1. 登录页已打开。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 用户名 `admin_op`，密码 `wrong`，点击登录 | 页面顶部弹出红色 message："用户名或密码错误" |
| 2 | `AdminWeb` | 仍停留在 `/login` | URL 未变 |
| 3 | `AdminWeb` | 用户名/密码均为空 | 表单显示字段下方红色校验文字"请输入用户名" |

**【数据清理】**
- 无。

---

## TC-AUTH-00003：路由守卫 - 未登录访问受保护页
**【元数据】**
- **归属模块**：`AUTH`
- **测试类型**：`Security`
- **回归级别**：`P0`

**【前置条件】**
1. localStorage 无 admin_token。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 访问 `/rooms` | 被重定向到 `/login?redirect=/rooms` |
| 2 | `AdminWeb` | 登录成功 | 自动跳转回 `/rooms` |
| 3 | `AdminWeb` | 登录后直接访问 `/login` | 被重定向到 `/dashboard` |

**【数据清理】**
- 无。

---

## TC-AUTH-00004：Token 过期 - 自动退出
**【元数据】**
- **归属模块**：`AUTH`
- **测试类型**：`Security`
- **回归级别**：`P1`

**【前置条件】**
1. 已登录，手动将 localStorage 中 admin_token 替换为过期 token。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 刷新页面或触发任意 API 请求 | AXIOS 拦截器收到 HTTP 401 |
| 2 | `AdminWeb` | 页面行为 | 清空本地 token；Antd message 弹出"登录已过期，请重新登录"；0.5s 后跳转 `/login` |

**【数据清理】**
- 无。

---

## TC-AUTH-00005：i18n 中英切换
**【元数据】**
- **归属模块**：`AUTH`
- **测试类型**：`Compatibility`
- **回归级别**：`P2`

**【前置条件】**
1. 登录页打开。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `AdminWeb` | 右上角"语言"下拉选"English" | 页面文本瞬时切换为英文：`Username` / `Password` / `Login` |
| 2 | `AdminWeb` | 切换回"简体中文" | 文本恢复中文 |
| 3 | `AdminWeb` | 刷新页面 | 语言偏好持久化（从 localStorage 读取） |

**【数据清理】**
- 无。
