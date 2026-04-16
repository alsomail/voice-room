import { webEnv } from '../core/config/env';
import { joinConfiguredUrl } from '../lib/url';

export async function apiClient(path: string, init?: RequestInit) {
  return fetch(joinConfiguredUrl(webEnv.apiBaseUrl, path), init);
}
