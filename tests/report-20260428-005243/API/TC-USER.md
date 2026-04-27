# TC-USER API — Admin 用户管理 测试报告

> 当前状态机：负责人 E2E | 状态 ✅ PASS | 修复轮次 3/5

**执行时间**：2026-04-28  
**执行环境**：local (AppServer :3000 + AdminServer :3001 + Postgres + Redis)  
**总计**：5 通过 / 0 失败 / 0 阻塞

## 用例结果

| 用例 ID | 描述 | 结果 |
|---------|------|------|
| TC-USER-00001 | 列表 - 分页/检索/XSS 安全 @prod-safe | ✅ PASS |
| TC-USER-00002 | 详情 - 含钱包/流水/设备 @prod-safe | ✅ PASS |
| TC-USER-00003 | 封禁用户 - 临时/永久 + 审计 + WS 踢下线 | ✅ PASS |
| TC-USER-00004 | 非法参数 + 重复封禁幂等 | ✅ PASS |
| TC-USER-00005 | 解封 - 状态恢复 + 审计 | ✅ PASS |

---

### 🛠️ TDD 修复记录 (Round 3/5)

- **排障 SOP 执行确认**：是
- **Bug 现象 (Phenomenon)**：TC-USER-00003/00005 在全套件并发执行时随机失败，ban/unban 状态与其他套件的 USER_A 操作冲突
- **根本原因 (Root Cause)**：TC-AUTH-00008 同时使用 USER_A token 做有效 token 验证；TC-USER ban USER_A 后，TC-AUTH-00008 得到 401 而非 200
- **修复方案 (Solution)**：
  - `tests/scripts/API/TC-USER.spec.ts`: 添加 `test.describe.configure({ mode: 'serial' })` 确保 ban/unban 串行
  - `tests/scripts/API/TC-AUTH.spec.ts`: TC-AUTH-00008 改用 `E2E_USER_B_TOKEN`（非封禁目标 USER_B）
