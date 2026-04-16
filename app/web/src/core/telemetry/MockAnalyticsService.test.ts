import { describe, expect, it, vi } from 'vitest';

import { MockAnalyticsService } from './MockAnalyticsService';
import { createTelemetryBaseContext } from './telemetryContext';

describe('MockAnalyticsService', () => {
  it('trackEvent injects device and environment context automatically', () => {
    const sink = vi.fn();
    const analytics = new MockAnalyticsService({
      sink,
      getBaseContext: () => ({
        device_id: 'device-123',
        os_version: 'macOS 15',
        network_type: 'wifi',
        locale: 'ar-SA',
        timezone: 'Asia/Riyadh',
        environment: 'development',
        analytics_endpoint: 'https://analytics.dev.internal',
      }),
    });

    analytics.trackEvent('room_opened', {
      room_id: 'room-1',
      source: 'homepage',
    });

    expect(sink).toHaveBeenCalledWith({
      eventName: 'room_opened',
      payload: {
        device_id: 'device-123',
        os_version: 'macOS 15',
        network_type: 'wifi',
        locale: 'ar-SA',
        timezone: 'Asia/Riyadh',
        environment: 'development',
        analytics_endpoint: 'https://analytics.dev.internal',
        room_id: 'room-1',
        source: 'homepage',
      },
    });
  });

  it('trackEvent does not allow payload to override injected telemetry context', () => {
    const sink = vi.fn();
    const analytics = new MockAnalyticsService({
      sink,
      getBaseContext: () => ({
        device_id: 'device-123',
        os_version: 'macOS 15',
        network_type: 'wifi',
        locale: 'ar-SA',
        timezone: 'Asia/Riyadh',
        environment: 'development',
        analytics_endpoint: 'https://analytics.dev.internal',
      }),
    });

    analytics.trackEvent('room_opened', {
      device_id: 'forged-device',
      environment: 'forged-env',
      analytics_endpoint: 'https://evil.example',
    });

    expect(sink).toHaveBeenCalledWith({
      eventName: 'room_opened',
      payload: {
        device_id: 'device-123',
        os_version: 'macOS 15',
        network_type: 'wifi',
        locale: 'ar-SA',
        timezone: 'Asia/Riyadh',
        environment: 'development',
        analytics_endpoint: 'https://analytics.dev.internal',
      },
    });
  });

  it('setUserProperties preserves caller fields and injects base context', () => {
    const sink = vi.fn();
    const analytics = new MockAnalyticsService({
      sink,
      getBaseContext: () => ({
        device_id: 'device-123',
        os_version: 'macOS 15',
        network_type: 'wifi',
        locale: 'en-US',
        timezone: 'UTC',
        environment: 'production',
        analytics_endpoint: 'https://analytics.prod.internal',
      }),
    });

    analytics.setUserProperties({
      user_id: 'user-1',
      room_id: 'room-9',
    });

    expect(sink).toHaveBeenCalledWith({
      eventName: 'user_properties_updated',
      payload: {
        device_id: 'device-123',
        os_version: 'macOS 15',
        network_type: 'wifi',
        locale: 'en-US',
        timezone: 'UTC',
        environment: 'production',
        analytics_endpoint: 'https://analytics.prod.internal',
        user_id: 'user-1',
        room_id: 'room-9',
      },
    });
  });

  it('createTelemetryBaseContext reuses the same device id across calls', () => {
    const first = createTelemetryBaseContext();
    const second = createTelemetryBaseContext();

    expect(second.device_id).toBe(first.device_id);
  });

  it('createTelemetryBaseContext does not use a shared fallback device id', () => {
    const context = createTelemetryBaseContext();

    expect(context.device_id).not.toBe('web-device');
  });
});
