import { webEnv } from '../config/env';

export interface TelemetryBaseContext {
  device_id: string;
  os_version: string;
  network_type: string;
  locale: string;
  timezone: string;
  environment: string;
  analytics_endpoint: string;
  user_id?: string;
  room_id?: string;
}

let cachedDeviceId: string | undefined;

export function createTelemetryBaseContext(): TelemetryBaseContext {
  const nav = typeof navigator === 'undefined' ? undefined : navigator;
  const locale = nav?.language ?? 'en-US';
  const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone ?? 'UTC';
  const connection = nav && 'connection' in nav ? nav.connection : undefined;
  const networkType =
    typeof connection === 'object' &&
    connection !== null &&
    'effectiveType' in connection &&
    typeof connection.effectiveType === 'string'
      ? connection.effectiveType
      : 'unknown';

  return {
    device_id: resolveDeviceId(),
    os_version: resolveOsVersion(nav),
    network_type: networkType,
    locale,
    timezone,
    environment: import.meta.env.MODE,
    analytics_endpoint: webEnv.analyticsEndpoint,
  };
}

function resolveDeviceId() {
  if (cachedDeviceId) {
    return cachedDeviceId;
  }

  const storageKey = 'voice-room-device-id';

  if (typeof localStorage !== 'undefined') {
    try {
      const storedDeviceId = localStorage.getItem(storageKey);

      if (storedDeviceId) {
        cachedDeviceId = storedDeviceId;
        return storedDeviceId;
      }
    } catch {
      // Ignore storage access failures and keep an in-memory identifier.
    }
  }

  const deviceId = generateDeviceId();
  cachedDeviceId = deviceId;

  if (typeof localStorage !== 'undefined') {
    try {
      localStorage.setItem(storageKey, deviceId);
    } catch {
      // Ignore storage access failures and keep an in-memory identifier.
    }
  }

  return deviceId;
}

function resolveOsVersion(nav: Navigator | undefined) {
  if (!nav) {
    return 'unknown';
  }

  const navWithUserAgentData = nav as Navigator & {
    userAgentData?: {
      platform?: string;
      platformVersion?: string;
    };
  };

  const platform = navWithUserAgentData.userAgentData?.platform ?? nav.platform;
  const platformVersion = navWithUserAgentData.userAgentData?.platformVersion;

  return [platform, platformVersion].filter(Boolean).join(' ') || 'unknown';
}

function generateDeviceId() {
  if (
    typeof crypto !== 'undefined' &&
    typeof crypto.randomUUID === 'function'
  ) {
    return crypto.randomUUID();
  }

  return `web-device-${Date.now().toString(36)}-${Math.random()
    .toString(36)
    .slice(2, 10)}`;
}
