export const webEnv = {
  apiBaseUrl: import.meta.env.VITE_API_BASE_URL,
  wsUrl: import.meta.env.VITE_WS_URL,
  analyticsEndpoint: import.meta.env.VITE_ANALYTICS_ENDPOINT,
} as const;
