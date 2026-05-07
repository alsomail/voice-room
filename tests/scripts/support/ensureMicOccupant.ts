/**
 * BUG-MIC-SEAT-SEED（Round 6）
 *
 * 房间麦位 (mic_seats) 是 Rust Server 的进程内状态（DashMap），无 DB 表，
 * 无法通过 SQL seed 预占。本工具用 WS 信令在测试启动前让一个种子用户上麦：
 *
 *   1. 用 E2E_USER_B_TOKEN（或 E2E_VALID_TOKEN 兜底）打开 WS
 *   2. JoinRoom → TakeMic(mic_index)
 *   3. 返回 dispose() 函数；测试结束时 LeaveMic + close
 *
 * 注意：mic 占位依赖 WS 连接活着；调用方必须在测试结束前调用 dispose()，
 * 否则 Server 端在连接断开时会自动 leave_mic_slot。
 *
 * 用法：
 * ```ts
 * const occupant = await ensureMicOccupant({ wsUrl, token, roomId, micIndex: 0 });
 * try {
 *   // ... run test
 * } finally {
 *   await occupant?.dispose();
 * }
 * ```
 *
 * 任一步骤失败时返回 null（best-effort），调用方应继续执行测试并通过 aiBoolean
 * 跳过相关断言；这样在 token 未配置或 server 未启动时不会让用例硬挂。
 */
import WebSocket from 'ws';

export interface MicOccupant {
  /** 关闭 WS 并释放麦位（best-effort） */
  dispose: () => Promise<void>;
  /** 实际占用的 mic_index */
  micIndex: number;
  /** 占用者 user_id（来自 token sub claim，仅日志用） */
  userId?: string;
}

export interface EnsureMicOccupantOptions {
  wsUrl: string;
  token: string;
  roomId: string;
  /** 默认 0；占用失败会自动尝试 1..8 */
  micIndex?: number;
  /** 等待响应的超时（ms），默认 4000 */
  timeoutMs?: number;
}

function decodeSub(token: string): string | undefined {
  try {
    const parts = token.split('.');
    if (parts.length < 2) return undefined;
    const json = JSON.parse(Buffer.from(parts[1], 'base64url').toString('utf8'));
    return typeof json.sub === 'string' ? json.sub : undefined;
  } catch {
    return undefined;
  }
}

async function recv(
  ws: WebSocket,
  match: (m: any) => boolean,
  timeoutMs: number,
): Promise<any> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      ws.off('message', handler);
      reject(new Error('timeout'));
    }, timeoutMs);
    const handler = (data: WebSocket.RawData) => {
      try {
        const msg = JSON.parse(data.toString());
        if (match(msg)) {
          clearTimeout(timer);
          ws.off('message', handler);
          resolve(msg);
        }
      } catch {
        /* ignore parse error */
      }
    };
    ws.on('message', handler);
  });
}

export async function ensureMicOccupant(
  opts: EnsureMicOccupantOptions,
): Promise<MicOccupant | null> {
  const { wsUrl, token, roomId, timeoutMs = 4000 } = opts;
  if (!wsUrl || !token || !roomId) {
    console.warn('[ensureMicOccupant] missing wsUrl/token/roomId — skip seed');
    return null;
  }

  const url = `${wsUrl}?token=${encodeURIComponent(token)}`;
  const ws = new WebSocket(url);
  // Keepalive: 每 20s 发一次 Ping，防止 30s 心跳超时断连
  // （服务端心跳超时时会先把 connection 从 registry 移除，导致 do_leave_room 早退，
  //   麦位无法自动释放，形成陈旧占位。加 Ping 彻底避免该竞态。）
  let keepaliveTimer: ReturnType<typeof setInterval> | null = null;

  try {
    await new Promise<void>((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error('ws connect timeout')), timeoutMs);
      ws.once('open', () => { clearTimeout(timer); resolve(); });
      ws.once('error', (err) => { clearTimeout(timer); reject(err); });
    });

    // 启动心跳保活（连接建立后立即开始）
    keepaliveTimer = setInterval(() => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: 'Ping', msg_id: `keepalive_${Date.now()}` }));
      }
    }, 20_000);

    // JoinRoom
    const joinMsgId = `seed_join_${Date.now()}`;
    ws.send(JSON.stringify({ type: 'JoinRoom', payload: { room_id: roomId }, msg_id: joinMsgId }));
    const joinResult = await recv(
      ws,
      (m) => m.type === 'JoinRoomResult' || m.type === 'JoinedRoom' || m.type === 'RoomState',
      timeoutMs,
    );
    if (joinResult.type === 'JoinRoomResult' && joinResult.code !== 0) {
      throw new Error(`JoinRoom failed code=${joinResult.code}`);
    }

    // 尝试候选麦位（默认 micIndex 优先；失败再依次尝试 1..8）
    const preferred = opts.micIndex ?? 0;
    const candidates = [preferred, ...Array.from({ length: 9 }, (_, i) => i).filter((i) => i !== preferred)];
    let occupied = -1;
    for (const idx of candidates) {
      const takeMsgId = `seed_take_${idx}_${Date.now()}`;
      ws.send(JSON.stringify({ type: 'TakeMic', payload: { mic_index: idx }, msg_id: takeMsgId }));
      try {
        const ack = await recv(ws, (m) => m.msg_id === takeMsgId, timeoutMs);
        if (ack.code === 0) {
          occupied = idx;
          console.log(`[ensureMicOccupant] ✅ WS 占麦成功 mic_index=${idx}`);
          break;
        } else {
          console.warn(`[ensureMicOccupant] TakeMic(${idx}) code=${ack.code} msg=${ack.message ?? ''}`);
        }
      } catch {
        /* 超时则尝试下一个 */
      }
    }
    if (occupied < 0) throw new Error('no free mic slot');

    const userId = decodeSub(token);
    return {
      micIndex: occupied,
      userId,
      dispose: async () => {
        if (keepaliveTimer) { clearInterval(keepaliveTimer); keepaliveTimer = null; }
        try {
          if (ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify({ type: 'LeaveMic', msg_id: `seed_leave_${Date.now()}` }));
            await new Promise((r) => setTimeout(r, 300));
          }
        } catch {
          /* best-effort */
        } finally {
          try { ws.close(); } catch { /* */ }
        }
      },
    };
  } catch (err) {
    if (keepaliveTimer) { clearInterval(keepaliveTimer); keepaliveTimer = null; }
    try { ws.close(); } catch { /* */ }
    console.warn('[ensureMicOccupant] seed mic failed (best-effort):', (err as Error).message);
    return null;
  }
}
