# TC-GIFT WEB — 礼物管理 测试报告

> 当前状态机：负责人 E2E | 状态 🚫 BLOCK | 修复轮次 3/5

**执行时间**：2026-04-28  
**执行环境**：local (React Web :5173 + AdminServer :3001)  
**总计**：0 通过 / 0 失败 / 2 阻塞（BUG-WEB-001）

## 用例结果

| 用例 ID | 描述 | 结果 |
|---------|------|------|
| TC-GIFT-00001 | 列表 + 筛选 | 🚫 BLOCK |
| TC-GIFT-00002 | 新增礼物 + 图片白名单 + CRUD | 🚫 BLOCK |

## 阻塞业务 Bug

### BUG-WEB-001: Midscene AI 缓存未命中 / OPENAI_API_KEY 未配置

- **影响用例**：全部 WEB 测试（18 个用例）
- **现象**：Midscene AI 视觉测试框架在 `MIDSCENE_CACHE=1` 模式下缓存未命中，所有 WEB 测试超时失败
- **建议**：需架构师介入，提供 `OPENAI_API_KEY` 或预录制 Midscene 缓存文件
