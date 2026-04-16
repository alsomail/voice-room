import { webEnv } from '../config/env';
import { joinConfiguredUrl } from '../../lib/url';

export function createRoomSocket(path = '/room') {
  return new WebSocket(joinConfiguredUrl(webEnv.wsUrl, path));
}
