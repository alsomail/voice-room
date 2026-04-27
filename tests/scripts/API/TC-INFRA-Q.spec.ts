/**
 * 测试套件：e2e-up.sh 端口冲突检测（T-0000Q）
 * 用例：I-1 (npm scripts 集成)
 */
import { test, expect } from '@playwright/test';
import { spawn } from 'child_process';
import { execSync } from 'child_process';

test.describe('TC-INFRA-Q - e2e-up 端口冲突检测', () => {
  test('I-1: e2e-up.sh 端口冲突时阻止启动并显示清晰错误', async () => {
    test.skip(process.platform === 'win32', 'Windows 不支持 nc');
    
    // 占用 PostgreSQL 端口 5432
    const ncProcess = spawn('nc', ['-l', '5432']);
    
    try {
      // 等待端口绑定
      await new Promise(resolve => setTimeout(resolve, 500));
      
      // 运行 e2e-up.sh，预期失败
      let stderr = '';
      let stdout = '';
      let exitCode = 0;
      
      try {
        const result = execSync('bash scripts/dev/e2e-up.sh', {
          encoding: 'utf-8',
          cwd: process.cwd(),
          stdio: ['ignore', 'pipe', 'pipe'],
          timeout: 10000
        });
        stdout = result;
      } catch (e: any) {
        stderr = e.stderr || '';
        stdout = e.stdout || '';
        exitCode = e.status || 1;
      }
      
      const output = stdout + stderr;
      
      // 验证退出码非 0
      expect(exitCode).not.toBe(0);
      
      // 验证包含端口冲突错误信息
      expect(output.toLowerCase()).toMatch(/port.*5432.*already in use|✗.*5432/i);
      
      // 验证包含 PID 信息
      expect(output).toMatch(/PID \d+/);
      
      // 验证包含 ERROR 标识
      expect(output).toMatch(/ERROR/);
      
      // 验证未启动 docker compose（如果启动了，后续会有成功消息）
      expect(output).not.toMatch(/OK — 5 端就绪/);
      
    } finally {
      // 清理
      ncProcess.kill('SIGKILL');
    }
  });

  test('I-2: e2e-up.sh 所有端口空闲时正常启动（可选，需要完整环境）', () => {
    test.skip(true, '需要完整 E2E 环境，手动测试');
    // 手动测试命令：npm run e2e:up
    // 预期：端口检测通过 → docker compose 成功启动 → 服务健康检查通过
  });
});
