/**
 * T-0000H E2E envLoader / globalSetup 共享类型。
 * 字段顺序与 T-0000E §2.7、T-0000F §2.3 24 字段表一一对应。
 */

export type E2EProfile = 'local' | 'staging' | 'prod';

export interface E2EEnv {
  profile: E2EProfile;
  allowWrites: boolean;

  // HTTP / WS 端点（4 项必填，所有 profile）
  appServerBaseUrl: string;
  adminServerBaseUrl: string;
  adminWebUrl: string;
  appWsUrl: string;

  // 仅 local 必填
  databaseUrl?: string;
  redisUrl?: string;

  // Android 用例条件必填
  androidAppId?: string;

  // Token 套件（staging/prod 必填；local 由 seed 回填，加载阶段允许空）
  tokens: {
    valid: string;
    expired: string;
    admin: string;
    op: string;
    cs: string;
    fin: string;
    expiredAdmin: string;
  };

  // Seed 回填 ID 三件套
  ids: {
    roomId: string;
    userAId: string;
    userBId: string;
  };

  // Midscene LLM
  midscene: {
    apiKey: string;
    modelName: string;
    baseUrl?: string;
    cache: boolean;
  };

  // CI 软门禁
  ciReady: boolean;

  // T-0000P 扩展：Azure / 自定义 baseURL 可选字段（internal，由 writeProcessEnv 透传）
  _azureEndpoint?: string;
  _azureApiKey?: string;
}
