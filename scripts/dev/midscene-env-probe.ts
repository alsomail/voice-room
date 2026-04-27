#!/usr/bin/env tsx
/**
 * T-0000P Midscene env 注入链 dry-run 验证脚本
 *
 * 用途：在无法启动完整 E2E 环境时，快速验证 Midscene API Key 是否正确注入。
 * 使用：zsh -ic 'cd /path/to/repo && npx tsx scripts/dev/midscene-env-probe.ts'
 * 安全：仅打印 Key 长度和首尾各 4 字符的脱敏形式，绝不打印完整 Key。
 */

import { loadE2EEnv, writeProcessEnv } from '../../tests/scripts/support/envLoader';

function maskApiKey(key?: string): string {
  if (!key || key.length < 12) return '[EMPTY or TOO SHORT]';
  const prefix = key.substring(0, 4);
  const suffix = key.substring(key.length - 4);
  const maskedLen = key.length - 8;
  return `${prefix}${'*'.repeat(maskedLen)}${suffix}`;
}

try {
  console.log('[Midscene env probe] Loading E2E env...\n');

  const env = loadE2EEnv();
  writeProcessEnv(env);

  console.log('✅ E2E env loaded successfully\n');
  console.log('Profile:', env.profile);
  console.log('\n--- Midscene Config ---');
  console.log('MIDSCENE_MODEL_API_KEY (from env):', maskApiKey(env.midscene.apiKey));
  console.log('  Length:', env.midscene.apiKey.length);
  console.log('  Status:', env.midscene.apiKey ? '✅ SET' : '⚠️  EMPTY (WEB tests will skip)');
  console.log('MIDSCENE_MODEL_NAME:', env.midscene.modelName);
  console.log('MIDSCENE_MODEL_BASE_URL:', env.midscene.baseUrl ?? '[DEFAULT]');
  console.log('MIDSCENE_CACHE:', env.midscene.cache);

  console.log('\n--- Process.env Injection (after writeProcessEnv) ---');
  console.log('process.env.MIDSCENE_MODEL_API_KEY:', maskApiKey(process.env.MIDSCENE_MODEL_API_KEY));
  console.log('process.env.OPENAI_API_KEY:', maskApiKey(process.env.OPENAI_API_KEY));
  console.log('process.env.MIDSCENE_MODEL_BASE_URL:', process.env.MIDSCENE_MODEL_BASE_URL ?? '[UNSET]');

  if (env._azureEndpoint) {
    console.log('\n--- Azure OpenAI (Optional) ---');
    console.log('AZURE_OPENAI_ENDPOINT:', env._azureEndpoint);
    console.log('AZURE_OPENAI_API_KEY:', maskApiKey(env._azureApiKey));
  }

  console.log('\n✅ Midscene env injection chain verified.');
  console.log('   Next: Run `npm run e2e:local` or specific WEB test to validate.');

} catch (err) {
  console.error('\n❌ Failed to load E2E env:');
  console.error((err as Error).message);
  process.exit(1);
}
