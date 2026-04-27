# TC-AUTH WEB — 管理员登录 测试报告

> 当前状态机：负责人 E2E | 状态 🚫 BLOCK | 修复轮次 3/5

**执行时间**：2026-04-28  
**执行环境**：local (React Web :5173 + AdminServer :3001)  
**总计**：0 通过 / 0 失败 / 5 阻塞（BUG-WEB-001）

## 用例结果

| 用例 ID | 描述 | 结果 |
|---------|------|------|
| TC-AUTH-00001 | 登录页 UI + 记住用户名 | 🚫 BLOCK |
| TC-AUTH-00002 | 登录失败 - 错误凭证 + 表单校验 | 🚫 BLOCK |
| TC-AUTH-00003 | 路由守卫 - 未登录重定向 | 🚫 BLOCK |
| TC-AUTH-00004 | Token 过期自动退出 | 🚫 BLOCK |
| TC-AUTH-00005 | i18n 中英切换 + 持久化 | 🚫 BLOCK |

## 阻塞业务 Bug

### BUG-WEB-001: Midscene AI 缓存未命中 / OPENAI_API_KEY 未配置

- **影响用例**：全部 WEB 测试（18 个用例）
- **现象**：Midscene AI 视觉测试框架在 `MIDSCENE_CACHE=1` 模式下缓存未命中，且无 `OPENAI_API_KEY`，所有 WEB 测试在 15s timeout 内失败
- **位置**：`midscene_run/default.cache.yaml` — 缺少对应的 AI 断言缓存条目
- **建议**：需架构师介入，提供 `OPENAI_API_KEY` 或预录制 Midscene 缓存文件后重新执行 WEB 套件
