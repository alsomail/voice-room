# TC-USER WEB — 用户管理 测试报告

> 当前状态机：负责人 E2E | 状态 🚫 BLOCK | 修复轮次 3/5

**执行时间**：2026-04-28  
**执行环境**：local (React Web :5173 + AdminServer :3001)  
**总计**：0 通过 / 0 失败 / 3 阻塞（BUG-WEB-001）

## 用例结果

| 用例 ID | 描述 | 结果 |
|---------|------|------|
| TC-USER-00001 | 列表 - 分页/搜索/角色权限 | 🚫 BLOCK |
| TC-USER-00002 | 详情抽屉 + 封禁 E2E 多端闭环 | 🚫 BLOCK |
| TC-USER-00003 | 解封弹窗 - 原因必填 + 二次确认 | 🚫 BLOCK |

## 阻塞业务 Bug

### BUG-WEB-001: Midscene AI 缓存未命中 / OPENAI_API_KEY 未配置

- **影响用例**：全部 WEB 测试（18 个用例）
- **现象**：Midscene AI 视觉测试框架在 `MIDSCENE_CACHE=1` 模式下缓存未命中，所有 WEB 测试超时失败
- **建议**：需架构师介入，提供 `OPENAI_API_KEY` 或预录制 Midscene 缓存文件
