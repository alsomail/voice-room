### 🛠️ TDD 修复记录 (Round {N}/5)
- **排障 SOP 执行确认**：[是/否]（是否已读取 `/doc/DEBUG_SOP.md`）
- **Bug 现象 (Phenomenon)**：[简述观察到的报错，如：AppServer 抛出 500 空指针]
- **根本原因 (Root Cause)**：[代码层面的原因，如：在 `payment.ts` 中未对 `user.balance` 进行判空]
- **修复方案 (Solution)**：[简述你修改了哪些文件和逻辑]
  - `src/services/payment.ts`: 增加了 `user.balance ?? 0` 的兜底处理。