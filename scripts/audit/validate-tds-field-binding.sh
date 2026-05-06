#!/usr/bin/env bash
# validate-tds-field-binding.sh
# 验收 T-00107：TDS 字段级回填完成度
# 通过标准：P0=0, P1=0
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
AUDIT_SCRIPT="$REPO_ROOT/scripts/audit/protocol-binding-audit.ts"

echo "🔍 TDS 字段级回填验收（T-00107）"
echo "   Repo: $REPO_ROOT"
echo ""

if ! command -v npx &>/dev/null; then
  echo "❌ npx not found. Please install Node.js."
  exit 1
fi

OUTPUT=$(cd "$REPO_ROOT" && npx ts-node --project tsconfig.scripts.json "$AUDIT_SCRIPT" --dry-run 2>&1)
echo "$OUTPUT"

P0=$(echo "$OUTPUT" | grep "P0 Errors:" | awk '{print $3}')
P1=$(echo "$OUTPUT" | grep "P1 Warnings:" | awk '{print $3}')

echo ""
if [[ "$P0" == "0" && "$P1" == "0" ]]; then
  echo "✅ T-00107 验收通过：P0=$P0, P1=$P1"
  exit 0
else
  echo "❌ T-00107 验收未通过：P0=$P0, P1=$P1"
  exit 1
fi
