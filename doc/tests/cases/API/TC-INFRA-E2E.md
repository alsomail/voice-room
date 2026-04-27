# 测试套件：INFRA-E2E 测试基建（模块 9 · QA Foundation）

> **需求模糊点 (Ambiguity Notes)**：
> - `staging` profile 真实远端凭据由 SRE 后续提供；当前用 `*.example.com` 占位，相关测试用例对 `staging` 实跑场景以「凭据已填」为前提，未填时按 `MissingEnvError` 用例覆盖。
> - `E2E_TOKEN_TTL_SECONDS` 默认 `86400`（24h），过期 token 用 `90d 前签发的常量`，假设 `JWT_SECRET` 不变。
> - Android `assembleProdRelease` 验收以「编译通过 + APK 可同设备并存」为准，**不**校验签名/上架链路（O2/O5 已在 T-30050 §1.4 排除）。

覆盖 Task：T-0000E（主设计，本套件 SOP 锚点）、T-0000F（env 模板）、T-0000G（Seed/Reset/Preflight）、T-0000H（envLoader/globalSetup/Teardown）、T-00040（AppServer config）、T-10020（AdminServer config）、T-20020（Web 多 env）、T-30050（Android Flavor）、T-0000I（npm scripts）、T-0000J（baseURL/typo/@prod-safe）、T-0000K（Midscene 配置）、T-0000L（RUNBOOK 文档）。

---

## TC-INFRA-E2E-00001：根 `.env.example` 与 `tests/scripts/env/.env.{profile}.example` 字段集合完全一致
**【元数据】**
- **归属模块**：`INFRA/E2E`
- **测试类型**：`Integration`
- **回归级别**：`P0`
- **关联 Task**：T-0000F

**【前置条件】**
1. 已 checkout 最新 main 分支，工作区干净。
2. 仓库根存在 `.env.example`；`tests/scripts/env/` 目录下存在 `.env.local.example` `.env.staging.example` `.env.prod.example` 三个文件。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 执行 `grep -E '^[A-Z_]+=' .env.example \| awk -F= '{print $1}' \| sort -u > /tmp/root.keys` | 退出码 0；`/tmp/root.keys` 行数 ≥ 23（覆盖 T-0000F §2.3 全字段） |
| 2 | `Shell` | 对三档 `.env.{local,staging,prod}.example` 各执行同样 `grep \| awk \| sort` 输出 `/tmp/{p}.keys` | 三个文件均生成；行数与 `/tmp/root.keys` 一致 |
| 3 | `Shell` | 执行 `diff /tmp/root.keys /tmp/local.keys && diff /tmp/root.keys /tmp/staging.keys && diff /tmp/root.keys /tmp/prod.keys` | 三个 diff 全部退出码 0，无字段差异 |
| 4 | `Shell` | 执行 `grep '^E2E_ALLOW_WRITES=' tests/scripts/env/.env.prod.example` | 输出唯一一行 `E2E_ALLOW_WRITES=0`（prod 默认禁写） |
| 5 | `Shell` | 执行 `grep -E 'app_server_pwd' .env.example tests/scripts/env/*.example` | 退出码非 0（grep 无匹配），证明 typo 已根治 |
| 6 | `Shell` | 执行 `grep -E 'app_server_pass' .env.example` | 至少 1 行匹配，证明已使用统一字段名 |

**【数据清理】**
- 删除 `/tmp/root.keys` `/tmp/local.keys` `/tmp/staging.keys` `/tmp/prod.keys`。

---

## TC-INFRA-E2E-00002：真实 `.env*` 文件被 `.gitignore` 覆盖且 `*.example` 不被 ignore
**【元数据】**
- **归属模块**：`INFRA/E2E`
- **测试类型**：`Security`
- **回归级别**：`P0`
- **关联 Task**：T-0000F

**【前置条件】**
1. 仓库根 `.gitignore` 已存在并包含 E2E 段。
2. 当前未 commit 过 `tests/scripts/env/.env.local`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 创建临时文件：`echo "E2E_VALID_TOKEN=fake" > tests/scripts/env/.env.local` | 文件创建成功 |
| 2 | `Shell` | 执行 `git check-ignore -v tests/scripts/env/.env.local` | 退出码 0，输出包含 `.gitignore` 行号，证明被忽略 |
| 3 | `Shell` | 执行 `git check-ignore -v tests/scripts/env/.env.local.example` | 退出码 1（未被 ignore），证明 example 文件可入库 |
| 4 | `Shell` | 执行 `git check-ignore -v scripts/dev/.seed-output.env` | 退出码 0，证明 seed 回填产物被 ignore |

**【数据清理】**
- 删除步骤 1 创建的 `tests/scripts/env/.env.local`。

---

## TC-INFRA-E2E-00003：preflight.sh 五端健康检查全绿路径
**【元数据】**
- **归属模块**：`INFRA/E2E`
- **测试类型**：`Integration`
- **回归级别**：`P0`
- **关联 Task**：T-0000G

**【前置条件】**
1. `docker compose up -d` 已启动 PG + Redis；AppServer/AdminServer/Web 三个进程已启动并就绪。
2. `tests/scripts/env/.env.local` 已填好全部字段。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 执行 `E2E_PROFILE=local bash scripts/dev/preflight.sh` | 退出码 0；stdout 含 5 行 `[OK]` 标记（PG/Redis/AppServer/AdminServer/Web） |
| 2 | `Shell` | 计时执行 `time bash scripts/dev/preflight.sh` | 总耗时 ≤ 2 秒（real time），单项检查 ≤ 2s 上限生效 |
| 3 | `Shell` | 执行 `npm run preflight` | 退出码 0；输出与步骤 1 等价 |

**【数据清理】**
- 无。

---

## TC-INFRA-E2E-00004：preflight.sh 任一端异常时彩色定位 + 专属退出码
**【元数据】**
- **归属模块**：`INFRA/E2E`
- **测试类型**：`Integration`
- **回归级别**：`P0`
- **关联 Task**：T-0000G

**【前置条件】**
1. PG + Redis 正常；AppServer 进程**已停止**。
2. `.env.local` 已填好。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 执行 `bash scripts/dev/preflight.sh; echo "rc=$?"` | rc=13（AppServer 专属退出码，11=PG / 12=Redis / 13=AppServer / 14=AdminServer / 15=Web） |
| 2 | `Shell` | 观察 stdout | 包含红色 `[FAIL] AppServer` 标记 + 修复 hint（如 `cargo run -p server` 提示） |
| 3 | `Shell` | 启动 AppServer 后重跑 `bash scripts/dev/preflight.sh` | 退出码 0，全部 `[OK]` |
| 4 | `Shell` | 停止 Redis 容器后执行 `bash scripts/dev/preflight.sh` | 退出码 12 |

**【数据清理】**
- 重新启动停止的服务，恢复初始状态。

---

## TC-INFRA-E2E-00005：seed-e2e.sh 幂等执行 + ID 与 Token 回填 `.seed-output.env`
**【元数据】**
- **归属模块**：`INFRA/E2E`
- **测试类型**：`Integration`
- **回归级别**：`P0`
- **关联 Task**：T-0000G

**【前置条件】**
1. PG 已启动且业务表迁移完成。
2. `E2E_PROFILE=local`，`.env.local` 已填好 `JWT_SECRET` 等。
3. `scripts/dev/.seed-output.env` 文件不存在。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 执行 `npm run db:seed` | 退出码 0；终端输出包含 `[SEED] users/admins/rooms inserted` 之类成功标记 |
| 2 | `Shell` | 检查 `cat scripts/dev/.seed-output.env` | 文件存在；至少包含 `E2E_VALID_TOKEN=` `E2E_ADMIN_TOKEN=` `E2E_ROOM_ID=` `E2E_USER_A_ID=` `E2E_USER_B_ID=` 等 7+ 行非空字段 |
| 3 | `DB` | psql 执行 `SELECT count(*) FROM users WHERE phone IN ('+966500000900','+966500000901')` | 返回 `2` |
| 4 | `Shell` | **再次执行** `npm run db:seed` | 退出码 0；users 表行数不变（`ON CONFLICT DO UPDATE` 幂等） |
| 5 | `DB` | 执行 `SELECT count(*) FROM rooms WHERE id = '<E2E_ROOM_ID>'` | 返回 `1`，主键稳定 |
| 6 | `Shell` | 校验回填 token 可解码：`node -e "console.log(JSON.parse(Buffer.from(process.argv[1].split('.')[1],'base64')))" "$(grep E2E_VALID_TOKEN scripts/dev/.seed-output.env \| cut -d= -f2)"` | 输出包含 `sub` `exp` 字段，`exp` > 当前时间 |

**【数据清理】**
- 执行 `npm run db:reset` 恢复测试数据初始状态。
- 删除 `scripts/dev/.seed-output.env`。

---

## TC-INFRA-E2E-00006：seed-e2e.sh / reset-e2e.sh 在非 local profile 下拒绝执行
**【元数据】**
- **归属模块**：`INFRA/E2E`
- **测试类型**：`Security`
- **回归级别**：`P0`
- **关联 Task**：T-0000G

**【前置条件】**
1. PG 已启动。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 执行 `E2E_PROFILE=staging bash scripts/dev/seed-e2e.sh; echo "rc=$?"` | rc=21（非 local 拒执行专属退出码）；stderr 含 `refuse to seed on profile=staging` |
| 2 | `Shell` | 执行 `E2E_PROFILE=prod bash scripts/dev/reset-e2e.sh; echo "rc=$?"` | rc 非 0（21~24 范围），不触发任何 DELETE |
| 3 | `DB` | 执行 `SELECT count(*) FROM users` | 行数与执行前完全一致（幂等保护生效） |
| 4 | `Shell` | 执行 `E2E_PROFILE=local E2E_ALLOW_WRITES=0 bash scripts/dev/seed-e2e.sh; echo "rc=$?"` | rc 非 0；stderr 含 `E2E_ALLOW_WRITES=0` 拒执行提示 |

**【数据清理】**
- 无。

---

## TC-INFRA-E2E-00007：reset-e2e.sh 仅清测试数据，不影响业务表结构与非 E2E 行
**【元数据】**
- **归属模块**：`INFRA/E2E`
- **测试类型**：`Integration`
- **回归级别**：`P0`
- **关联 Task**：T-0000G

**【前置条件】**
1. 已执行 seed；存在 E2E 测试数据。
2. 业务表中预先插入一条**非 E2E** 用户：`INSERT INTO users(id, phone) VALUES (gen_random_uuid(),'+966599999999')`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 执行 `npm run db:reset` | 退出码 0 |
| 2 | `DB` | 执行 `SELECT count(*) FROM users WHERE phone IN ('+966500000900','+966500000901')` | 返回 `0`（E2E 用户已清空） |
| 3 | `DB` | 执行 `SELECT count(*) FROM users WHERE phone='+966599999999'` | 返回 `1`（非 E2E 数据未受影响） |
| 4 | `DB` | 执行 `\d users` 查表结构 | 表结构与 reset 前完全一致，未做 DROP/TRUNCATE |
| 5 | `Shell` | **再次执行** `npm run db:reset` | 退出码 0（幂等：无数据可删也不报错） |

**【数据清理】**
- `DELETE FROM users WHERE phone='+966599999999'`。

---

## TC-INFRA-E2E-00008：envLoader 缺关键字段时抛 `MissingEnvError` 并以退出码 78 终止
**【元数据】**
- **归属模块**：`INFRA/E2E`
- **测试类型**：`Functional`
- **回归级别**：`P0`
- **关联 Task**：T-0000H, T-0000F

**【前置条件】**
1. 临时备份 `tests/scripts/env/.env.staging`（如存在）；构造一份**故意缺失** `APP_SERVER_BASE_URL` 的 `.env.staging`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 执行 `E2E_PROFILE=staging npx playwright test tests/scripts/API/TC-AUTH.spec.ts; echo "rc=$?"` | rc=78（envLoader fail-fast 专属退出码） |
| 2 | `Shell` | 观察 stderr | 包含 `MissingEnvError` 类名 + 缺失字段名 `APP_SERVER_BASE_URL` + 文件路径 `tests/scripts/env/.env.staging` |
| 3 | `Shell` | 验证：进程在任何 `test()` 进入**之前**就退出 | stderr 不含任何用例标题（如 `TC-AUTH-00001`），证明 fail-fast 生效 |
| 4 | `Shell` | 运行 envLoader 单测 `npx tsx tests/scripts/support/__tests__/envLoader.test.ts`（或对应 runner 命令） | 退出码 0，覆盖 MissingEnvError 各分支 |

**【数据清理】**
- 恢复 `.env.staging` 原内容。

---

## TC-INFRA-E2E-00009：prod profile 下写操作类用例自动 skip 而非 fail
**【元数据】**
- **归属模块**：`INFRA/E2E`
- **测试类型**：`Security`
- **回归级别**：`P0`
- **关联 Task**：T-0000H, T-0000J

**【前置条件】**
1. `tests/scripts/env/.env.prod` 已填占位字段（实际不会请求远端，仅校验 fixture 行为）。
2. `E2E_ALLOW_WRITES=0`（prod 默认）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 执行 `npm run e2e:prod-smoke -- --list` | 退出码 0；输出仅包含带 `@prod-safe` 标签的用例（≥ 5 条） |
| 2 | `Shell` | 执行 `E2E_PROFILE=prod npx playwright test tests/scripts/API` | 写操作类用例（POST/PUT/DELETE）状态为 `skipped`，原因含 `prod-safe` 或 `writes disallowed` |
| 3 | `Shell` | 在 stdout 报告中检查 | 不含 `failed` 计数，全部为 `passed` 或 `skipped` |
| 4 | `Shell` | grep 验证：`grep -r '@prod-safe' tests/scripts/ \| wc -l` | 返回 ≥ 5（read-only smoke 用例已打标） |

**【数据清理】**
- 无。

---

## TC-INFRA-E2E-00010：globalSetup 顺序调用 envLoader → preflight → seed，注入 process.env
**【元数据】**
- **归属模块**：`INFRA/E2E`
- **测试类型**：`Integration`
- **回归级别**：`P0`
- **关联 Task**：T-0000H

**【前置条件】**
1. 五端服务全部启动；`.env.local` 已填好。
2. `scripts/dev/.seed-output.env` 不存在。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 执行 `npm run e2e:local -- tests/scripts/API/TC-AUTH.spec.ts --reporter=line` | 退出码 0 |
| 2 | `Shell` | 观察启动期 stdout 顺序 | 先 `[envLoader] loaded profile=local`，后 `[preflight] all 5 OK`，最后 `[seed] inserted users=2 admins=4 rooms=1` |
| 3 | `Shell` | 检查 `scripts/dev/.seed-output.env` | 文件由 globalSetup 自动产出，非空 |
| 4 | `Shell` | 测试结束后检查 `globalTeardown` 输出 | 包含 `[reset] done`，且非 local profile 下 reset 不被调用（可用 staging dry-run 验证） |
| 5 | `Shell` | 全程进程模型 | playwright 主线程一次性退出，无僵尸进程残留 |

**【数据清理】**
- 由 globalTeardown 自动 reset；删除 `scripts/dev/.seed-output.env`。

---

## TC-INFRA-E2E-00011：AppServer 缺关键 env 时启动 fail-fast 给出明确错误
**【元数据】**
- **归属模块**：`INFRA/E2E`
- **测试类型**：`Functional`
- **回归级别**：`P0`
- **关联 Task**：T-00040

**【前置条件】**
1. 进入 `app/server/` 目录；本机 PG + Redis 已启动。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 执行 `unset JWT_SECRET DATABASE_URL && APP_PROFILE=dev cargo run -p server; echo "rc=$?"` | 退出码非 0（如 78）；stderr 含 `missing required config: jwt.secret / database.url` 之类明确字段名 |
| 2 | `Shell` | 执行 `APP_PROFILE=staging cargo run -p server -- --check-config` | 加载链：`default.toml + staging.toml + env`；缺字段时报错指向 staging.toml |
| 3 | `Shell` | 注入完整 env 后执行 `APP_PROFILE=dev cargo run -p server` | 服务启动成功，监听端口；2 秒内输出 `listening on 0.0.0.0:3000` |
| 4 | `Shell` | 执行 `cargo test -p server` | 退出码 0，0 回归（保证 config 重构未破坏既有测试） |

**【数据清理】**
- Ctrl+C 关闭 server 进程。

---

## TC-INFRA-E2E-00012：AdminServer config 加载链对称 + dev 缺 REDIS_URL 时回落 NoopEventPublisher
**【元数据】**
- **归属模块**：`INFRA/E2E`
- **测试类型**：`Functional`
- **回归级别**：`P0`
- **关联 Task**：T-10020

**【前置条件】**
1. PG 已启动；Redis 故意停止或 `REDIS_URL` 未设置（验证 D-A1 契约）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 执行 `unset ADMIN_JWT_SECRET DATABASE_URL && ADMIN_PROFILE=dev cargo run -p admin-server; echo "rc=$?"` | 退出码 78；stderr 含缺失字段名 |
| 2 | `Shell` | 执行 `ADMIN_PROFILE=staging cargo run -p admin-server -- --check-config` | 加载链 `default + staging + env`；与 server 对称 |
| 3 | `Shell` | 设 `ADMIN_PROFILE=dev` 但不设 `REDIS_URL`，启动 admin-server | 服务启动成功；日志含 `[event] using NoopEventPublisher (REDIS_URL absent)` |
| 4 | `Shell` | 执行 `cargo test -p admin-server` | 退出码 0；既有 474 项测试全绿 |

**【数据清理】**
- 关闭进程。

---

## TC-INFRA-E2E-00013：Web `vite --mode staging` 加载 `.env.staging` 且 apiClient 无硬编码默认值
**【元数据】**
- **归属模块**：`INFRA/E2E`
- **测试类型**：`Functional`
- **回归级别**：`P1`
- **关联 Task**：T-20020

**【前置条件】**
1. `app/web/` 下存在 `.env` `.env.test` `.env.staging` `.env.example`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 在 `app/web/` 执行 `npx vite build --mode staging` | 退出码 0；产物中 `VITE_ADMIN_API_BASE_URL` 取自 `.env.staging` |
| 2 | `Shell` | 执行 `grep -r "localhost:3001" app/web/src/` | 退出码非 0（apiClient 已删除硬编码默认值） |
| 3 | `Shell` | 临时删除 `.env.staging` 重跑 `vite build --mode staging` | 构建仍通过但 `import.meta.env.VITE_ADMIN_API_BASE_URL` 为空字符串；apiClient 在运行时报「VITE_ADMIN_API_BASE_URL is required」错误 |
| 4 | `Shell` | 执行 `npm test --prefix app/web` | 退出码 0，单测 0 回归 |

**【数据清理】**
- 恢复 `.env.staging`；清理 `app/web/dist`。

---

## TC-INFRA-E2E-00014：Android 三 flavor APK 同设备并存且 staging/prod 强制 HTTPS
**【元数据】**
- **归属模块**：`INFRA/E2E`
- **测试类型**：`Compatibility`
- **回归级别**：`P1`
- **关联 Task**：T-30050

**【前置条件】**
1. `app/android/` 已配置 `productFlavors { local; staging; prod }`。
2. 已连接一台 Android 模拟器（API 33+）。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 在 `app/android/` 执行 `./gradlew assembleLocalDebug assembleStagingRelease assembleProdRelease` | 退出码 0；三个 APK 全部产出 |
| 2 | `Android` | 依次 `adb install -r` 安装三个 APK | 三个包名 `com.voiceroom.local` `com.voiceroom.stg` `com.voiceroom` 在 `adb shell pm list packages \| grep voiceroom` 中同时存在（共 3 行） |
| 3 | `Shell` | 反编译/检查 `aapt dump badging app-staging-release.apk \| grep usesCleartextTraffic` | 输出无 `usesCleartextTraffic='true'`；同样验证 prod-release |
| 4 | `Shell` | 检查 `aapt dump badging app-local-debug.apk` | 含 `application-label`；`local` flavor 允许明文流量 |
| 5 | `Shell` | 故意将 staging flavor 的 `API_BASE_URL` 改为 `http://...` 重新构建 | 编译期 lint/check 失败（NetworkSecurityConfig 校验红线生效） |
| 6 | `Shell` | 执行 `./gradlew testLocalDebugUnitTest` | 退出码 0；既有单测 0 回归 |

**【数据清理】**
- `adb uninstall com.voiceroom.local com.voiceroom.stg com.voiceroom`。

---

## TC-INFRA-E2E-00015：六个 npm script 一键命令字面与契约一致
**【元数据】**
- **归属模块**：`INFRA/E2E`
- **测试类型**：`Functional`
- **回归级别**：`P1`
- **关联 Task**：T-0000I

**【前置条件】**
1. 仓库根 `package.json` 已上线 6 个 script。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 执行 `node -e "console.log(Object.keys(require('./package.json').scripts).sort().join(','))"` | 输出包含 `db:reset,db:seed,e2e:local,e2e:prod-smoke,e2e:staging,preflight` 6 个名称 |
| 2 | `Shell` | 执行 `npm run e2e:prod-smoke -- --list \| head -1` | 命令字面包含 `--grep "@prod-safe"`（双引号），与 T-0000J 标签 1:1 对账 |
| 3 | `Shell` | 执行 `npm run preflight`（全绿环境） | 退出码 0，1 秒内输出健康表 |
| 4 | `Shell` | 执行 `npm run db:seed; echo "rc=$?"` | 等价 `E2E_PROFILE=local E2E_ALLOW_WRITES=1 bash scripts/dev/seed-e2e.sh`，rc=0 |
| 5 | `Shell` | 在缺 cross-env 的环境下执行 `npm run e2e:local` | 退出码非 0，提示 `cross-env: command not found`（验证 devDep 必备） |

**【数据清理】**
- 执行 `npm run db:reset`。

---

## TC-INFRA-E2E-00016：Playwright `use.baseURL` 由 envLoader 注入 + 用例去硬编码
**【元数据】**
- **归属模块**：`INFRA/E2E`
- **测试类型**：`Integration`
- **回归级别**：`P1`
- **关联 Task**：T-0000J

**【前置条件】**
1. `.env.local` 中 `ADMIN_WEB_URL=http://localhost:5173`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 执行 `grep -rn "app_server_pwd" tests/ scripts/` | 退出码 1（grep 0 命中），typo 已根治 |
| 2 | `Shell` | 执行 `grep -rn "localhost:3000\|localhost:3001\|localhost:5173" tests/scripts/{API,E2E,WEB}/` | 退出码 1（无硬编码 URL fallback） |
| 3 | `Shell` | 执行 `grep -rn "import 'dotenv/config'" tests/scripts/{API,E2E,WEB}/` | 退出码 1（worker 内禁止再次 dotenv） |
| 4 | `Shell` | 执行 `grep -rn "page.goto('/" tests/scripts/WEB/` | 至少 5 条匹配（已用相对路径） |
| 5 | `Shell` | 在 `.env.local` 临时改 `ADMIN_WEB_URL=http://localhost:18888`（错误端口）后执行 `npm run e2e:local -- tests/scripts/WEB/TC-AUTH.spec.ts` | 用例 navigation 失败的报错 URL 中包含 `:18888`，证明 baseURL 来源链生效 |
| 6 | `Shell` | 执行 `grep -rn "@prod-safe" tests/scripts/` | ≥ 5 条 read-only smoke 用例已打标 |

**【数据清理】**
- 恢复 `ADMIN_WEB_URL` 为正确值。

---

## TC-INFRA-E2E-00017：Midscene API Key 缺失时 WEB 用例 skip 而非 fail，且 Key 不入 runtime json
**【元数据】**
- **归属模块**：`INFRA/E2E`
- **测试类型**：`Security`
- **回归级别**：`P1`
- **关联 Task**：T-0000K

**【前置条件】**
1. `tests/scripts/env/.env.local` 中 `MIDSCENE_MODEL_API_KEY=`（留空）。
2. 其他 env 字段已填。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 执行 `npm run e2e:local -- tests/scripts/WEB --reporter=list` | 退出码 0；WEB spec 全部状态 `skipped`，原因 `[MIDSCENE] api key missing — skipped` |
| 2 | `Shell` | 执行 `npm run e2e:local -- tests/scripts/API --reporter=list` | API 用例不受影响，正常 passed |
| 3 | `Shell` | 检查若存在 `tests/scripts/.e2e-runtime.json` | 文件中 `MIDSCENE_MODEL_API_KEY` 字段不存在或值为空字符串（不持久化 Key） |
| 4 | `Shell` | 设置 `MIDSCENE_MODEL_API_KEY=sk-test-fake` 后执行 WEB 用例 | 不再 skip；进入实际 PlaywrightAgent（用例可能因 fake key 而 fail，但不再 skip） |
| 5 | `Shell` | 执行 `grep -r "sk-" playwright-report/ test-results/ midscene_run/log/ 2>/dev/null` | 退出码 1 或匹配项被脱敏（`sk-***`），无明文 Key 泄漏 |
| 6 | `Shell` | 检查 `doc/tests/MIDSCENE_SETUP.md` | 文档存在；含「OpenAI 直连 / Azure / 中转」三形态字段映射表与 GitHub Actions Secret 注入示例 |

**【数据清理】**
- 恢复 `MIDSCENE_MODEL_API_KEY` 原值。
- 清理 `playwright-report/` `test-results/`。

---

## TC-INFRA-E2E-00018：E2E_RUNBOOK 冷启动 SOP 5 步全绿（新人验收）
**【元数据】**
- **归属模块**：`INFRA/E2E`
- **测试类型**：`Functional`
- **回归级别**：`P2`
- **关联 Task**：T-0000L

**【前置条件】**
1. 一台从未跑过本仓库 E2E 的干净开发机（Docker、Node、Rust 工具链已具备）。
2. 仓库已 `git clone`。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 打开 `doc/tests/E2E_RUNBOOK.md`，按「冷启动 5 步」逐步执行（cp env/install/up/seed/run） | 每步均有可直接复制的命令；总耗时 ≤ 5 分钟 |
| 2 | `Shell` | 完成后执行 `npm run e2e:local`（不限定子集） | 退出码 0；35 条 E2E 用例全绿（M1 里程碑判定） |
| 3 | `Shell` | 故意停止 PG 后再执行 `npm run preflight` | RUNBOOK 故障排查表能查到 `rc=11` 对应的修复指令（`docker compose up -d postgres`） |
| 4 | `Shell` | 检查 RUNBOOK 内容 | 含 staging/prod-safe 凭据获取流程占位（即便为 TBD，必须明确 owner） + GitHub Actions CI 接入示例 |
| 5 | `Shell` | 检查 `doc/tests/index.md` | 已链接到 `MIDSCENE_SETUP.md` 与 `E2E_RUNBOOK.md` |

**【数据清理】**
- 恢复 PG；执行 `npm run db:reset`；执行 `docker compose down -v` 清环境（如需）。

---

## TC-INFRA-E2E-00019：DB 角色隔离 - e2e_runner 有限权限不可访问敏感表
**【元数据】**
- **归属模块**：`INFRA/E2E`
- **测试类型**：`Security`
- **回归级别**：`P1`
- **关联 Task**：T-0000G

**【前置条件】**
1. 已通过 `init-db.sh` 创建 `e2e_runner` PG 角色。
2. 业务表 `admin_logs` `payment_orders` 等已存在。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `DB` | 以 `e2e_runner` 角色 psql：`SELECT count(*) FROM users` | 成功返回行数（具备读权限） |
| 2 | `DB` | 以 `e2e_runner` 角色：`INSERT INTO users(id,phone) VALUES (gen_random_uuid(),'+966500000777')` | 成功（具备写权限） |
| 3 | `DB` | 以 `e2e_runner` 角色：`SELECT * FROM admin_logs LIMIT 1` | 报错 `permission denied for table admin_logs` |
| 4 | `DB` | 以 `e2e_runner` 角色：`SELECT * FROM payment_orders LIMIT 1` | 报错 `permission denied for table payment_orders` |
| 5 | `Shell` | 执行 `bash scripts/dev/verify-permissions.sh` | 退出码 0，输出权限矩阵符合 grant-permissions.sql 期望 |

**【数据清理】**
- 以业务账号 `DELETE FROM users WHERE phone='+966500000777'`。

---

## TC-INFRA-E2E-00020：并发 E2E 执行不污染数据（端到端幂等闭环）
**【元数据】**
- **归属模块**：`INFRA/E2E`
- **测试类型**：`Performance`
- **回归级别**：`P2`
- **关联 Task**：T-0000G, T-0000H

**【前置条件】**
1. 五端正常；`.env.local` 已填好。

**【执行步骤与断言】**
| 步骤序号 | 目标端 | 操作动作 (Action) | 预期结果 (Assertion) |
| :------: | :----- | :---------------- | :------------------- |
| 1 | `Shell` | 执行 `npm run e2e:local -- --workers=4` | 退出码 0；用例耗时较 workers=1 显著下降（≥ 30%） |
| 2 | `DB` | 执行 `SELECT phone, count(*) FROM users WHERE phone LIKE '+96650000090%' GROUP BY phone HAVING count(*) > 1` | 0 行（确定性 ID + ON CONFLICT 保证不重复） |
| 3 | `Shell` | 连续执行 `npm run e2e:local` 三次 | 三次均退出码 0，无数据污染导致的间歇 fail |
| 4 | `Shell` | 关键 P0 接口 100 并发压测（如 `/api/auth/login`）响应时间 | P95 ≤ 2 秒 |

**【数据清理】**
- 由 globalTeardown 自动 reset。
