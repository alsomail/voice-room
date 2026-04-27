/**
 * 测试套件：CHAT 公屏聊天（API）
 * 用例来源：doc/tests/cases/API/TC-CHAT.md
 * 注：WS 层用例通过 ws 客户端或 AppServer 的 HTTP 调试端点驱动。
 */
import { test, expect } from '@playwright/test';
import { execSync } from 'child_process';
import WebSocket from 'ws';
import 'dotenv/config';

const APP_BASE = process.env.APP_SERVER_BASE_URL ?? 'http://localhost:3000';
const WS_BASE = process.env.APP_WS_URL ?? 'ws://localhost:3000/ws';
const TOKEN = process.env.E2E_VALID_TOKEN ?? '';
const TOKEN_B = process.env.E2E_USER_B_TOKEN ?? '';
const ROOM = process.env.E2E_ROOM_ID ?? '';
const psql = (s: string) =>
  execSync(`psql "${process.env.DATABASE_URL}" -tA -c "${s.replace(/"/g, '\\"')}"`, { encoding: 'utf-8' }).trim();

async function openWs(token: string): Promise<WebSocket> {
  const ws = new WebSocket(`${WS_BASE}?token=${token}`);
  await new Promise<void>((ok, ko) => {
    ws.once('open', () => ok());
    ws.once('error', ko);
  });
  return ws;
}

async function recvUntil(ws: WebSocket, match: (m: any) => boolean, timeoutMs = 3000): Promise<any> {
  return new Promise((ok, ko) => {
    const timer = setTimeout(() => ko(new Error('ws recv timeout')), timeoutMs);
    ws.on('message', (data) => {
      const m = JSON.parse(data.toString());
      if (match(m)) { clearTimeout(timer); ok(m); }
    });
  });
}

test.describe('TC-CHAT API - 公屏聊天', () => {
  test.skip(!TOKEN || !ROOM, '需要 E2E_VALID_TOKEN / E2E_ROOM_ID');

  test('TC-CHAT-00001: SendMessage 正常广播', async () => {
    const sender = await openWs(TOKEN);
    const receiver = TOKEN_B ? await openWs(TOKEN_B) : sender;
    sender.send(JSON.stringify({ type: 'JoinRoom', room_id: ROOM, msg_id: 'j1' }));
    if (sender !== receiver) receiver.send(JSON.stringify({ type: 'JoinRoom', room_id: ROOM, msg_id: 'j2' }));

    const msgId = `m_${Date.now()}`;
    sender.send(JSON.stringify({ type: 'SendMessage', room_id: ROOM, content: 'hello', msg_id: msgId }));
    const received = await recvUntil(receiver, (m) => m.type === 'ChatMessage' && m.msg_id === msgId);
    expect(received.content).toBe('hello');
    sender.close(); if (sender !== receiver) receiver.close();
  });

  test('TC-CHAT-00002: 内容长度边界 0/1/500/501', async () => {
    const ws = await openWs(TOKEN);
    ws.send(JSON.stringify({ type: 'JoinRoom', room_id: ROOM, msg_id: 'jj' }));
    for (const [len, expectOk] of [[0, false], [1, true], [500, true], [501, false]] as const) {
      const msgId = `m_${len}_${Date.now()}`;
      ws.send(JSON.stringify({ type: 'SendMessage', room_id: ROOM, content: 'a'.repeat(len), msg_id: msgId }));
      const reply = await recvUntil(ws, (m) => m.msg_id === msgId || m.type === 'Error');
      if (expectOk) expect(reply.type).toBe('ChatMessage');
      else expect(reply.type).toBe('Error');
    }
    ws.close();
  });

  test('TC-CHAT-00003: 敏感词过滤 / XSS', async () => {
    const ws = await openWs(TOKEN);
    ws.send(JSON.stringify({ type: 'JoinRoom', room_id: ROOM, msg_id: 'j3' }));
    const msgId = `m_${Date.now()}`;
    ws.send(JSON.stringify({ type: 'SendMessage', room_id: ROOM, content: '<script>alert(1)</script>fuck', msg_id: msgId }));
    const m = await recvUntil(ws, (x) => x.msg_id === msgId);
    expect(m.content).not.toContain('<script>');
    expect(m.content).toMatch(/\*+/);
    ws.close();
  });

  test('TC-CHAT-00004: CHAT_MUTED 禁言', async () => {
    const MUTED = process.env.E2E_MUTED_TOKEN ?? '';
    test.skip(!MUTED, '需要 E2E_MUTED_TOKEN');
    const ws = await openWs(MUTED);
    ws.send(JSON.stringify({ type: 'JoinRoom', room_id: ROOM, msg_id: 'jm' }));
    const msgId = `m_${Date.now()}`;
    ws.send(JSON.stringify({ type: 'SendMessage', room_id: ROOM, content: 'hi', msg_id: msgId }));
    const m = await recvUntil(ws, (x) => x.msg_id === msgId || x.type === 'Error');
    expect(m.type).toBe('Error');
    expect(m.code).toBe(40303);
    ws.close();
  });

  test('TC-CHAT-00005: msg_id 去重', async () => {
    const ws = await openWs(TOKEN);
    ws.send(JSON.stringify({ type: 'JoinRoom', room_id: ROOM, msg_id: 'jd' }));
    const msgId = `dup_${Date.now()}`;
    ws.send(JSON.stringify({ type: 'SendMessage', room_id: ROOM, content: 'once', msg_id: msgId }));
    ws.send(JSON.stringify({ type: 'SendMessage', room_id: ROOM, content: 'twice', msg_id: msgId }));
    await new Promise((r) => setTimeout(r, 1500));
    const cnt = psql(`SELECT count(*) FROM chat_messages WHERE msg_id='${msgId}'`);
    expect(cnt).toBe('1');
    ws.close();
  });
});
