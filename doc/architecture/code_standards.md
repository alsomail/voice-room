# 14. 代码规范与静态检查 (Linting & Formatting)

由于本 Monorepo 包含三种不同技术栈，必须采用**分层治理、各端独立**的静态检查策略。严禁在仓库根目录配置一个全局 ESLint/Prettier 去强管全部语言。

## 14.1 Web 端

工作目录：`/app/web/`

- 规范工具：ESLint + Prettier + TypeScript Compiler
- 配置文件：
  - `.eslintrc.cjs`
  - `.prettierrc`
- AI/开发约束：
  - 修改代码后必须执行 `npm run lint -- --fix`
  - 必须执行 `npm run format`
- 关键规则：
  - 开启 `eslint-plugin-react-hooks`
  - 禁止滥用 `any`

## 14.2 Server 端

工作目录：`/app/server/`

- 规范工具：`rustfmt` + `clippy`
- 配置文件：
  - `rustfmt.toml`
  - `.cargo/config.toml`（可选）
- AI/开发约束：
  - 修改后必须执行 `cargo fmt`
  - 必须执行 `cargo clippy -- -D warnings`
- 目标：Zero Clippy Warnings

## 14.3 Android 端

工作目录：`/app/android/`

- 规范工具：`ktlint` 或 `detekt`
- 配置文件：
  - `.editorconfig`
  - `build.gradle.kts`
- AI/开发约束：
  - 修改后必须执行 `./gradlew ktlintFormat` 或 `./gradlew detekt`
- 关键规则：
  - XML 禁止硬编码字符串
  - 强制使用 `strings.xml`

## 14.4 Git Hooks 与 CI

- 根目录使用 `husky` + `lint-staged`
- 根据改动路径触发对应子项目的检查命令：
  - `app/web/**/*.ts(x)` -> `cd app/web && npm run lint`
  - `app/server/**/*.rs` -> `cd app/server && cargo fmt --check`
  - `app/android/**/*.kt` -> `cd app/android && ./gradlew detekt`
