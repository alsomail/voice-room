#!/usr/bin/env bash
# check_no_sentry_imports.sh — T-30034 静态约束验证
#
# 验证业务层代码中没有直接 import io.sentry.*
# 唯一允许的例外：core/analytics/impl/ 目录下的 SentryAnalytics.kt
#
# 用法：
#   ./scripts/check_no_sentry_imports.sh
#
# 退出码：
#   0 — 通过（业务层无 io.sentry.* 直接 import）
#   1 — 失败（发现违规 import）

set -e

ANDROID_SRC="app/android/app/src/main/java/com/voice/room/android"

echo "🔍 检查业务层 import io.sentry.* 直接引用..."

# 仅检查真实的 import 语句（以 import 开头），排除注释中的提及
# 排除唯一允许的 impl/ 目录（SentryAnalytics.kt 所在位置）
VIOLATIONS=$(grep -r "^import io\.sentry\." \
  "${ANDROID_SRC}" \
  --include="*.kt" \
  --exclude-dir="impl" \
  -l 2>/dev/null || true)

if [ -n "$VIOLATIONS" ]; then
  echo "❌ 违规：以下文件直接 import 了 io.sentry.*（违反防腐层约束 T-30034）："
  echo "$VIOLATIONS"
  echo ""
  echo "解决方法：通过 AnalyticsPort 接口访问 Sentry 功能，禁止直接 import io.sentry.*"
  exit 1
fi

echo "✅ 通过：业务层无 io.sentry.* 直接 import（A34-01）"
exit 0
