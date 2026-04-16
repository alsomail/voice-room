import { describe, expect, it } from 'vitest';

import { joinConfiguredUrl } from './url';

describe('joinConfiguredUrl', () => {
  it('preserves api prefix when path starts with a slash', () => {
    expect(joinConfiguredUrl('https://api.example.com/api', '/v1/rooms')).toBe(
      'https://api.example.com/api/v1/rooms',
    );
  });

  it('preserves websocket prefix when joining room path', () => {
    expect(joinConfiguredUrl('wss://api.example.com/ws', '/room')).toBe(
      'wss://api.example.com/ws/room',
    );
  });

  it('rejects absolute urls that bypass the configured base', () => {
    expect(() =>
      joinConfiguredUrl('https://api.example.com/api', 'https://evil.example'),
    ).toThrow('Path must be relative to the configured base URL.');
  });

  it('rejects dot segments that escape the configured prefix', () => {
    expect(() =>
      joinConfiguredUrl('https://api.example.com/api', '../admin'),
    ).toThrow('Path must not contain dot segments.');
  });

  it('rejects absolute urls with leading whitespace', () => {
    expect(() =>
      joinConfiguredUrl('https://api.example.com/api', ' https://evil.example'),
    ).toThrow('Path must be relative to the configured base URL.');
  });

  it('rejects encoded dot segments that escape the configured prefix', () => {
    expect(() =>
      joinConfiguredUrl('https://api.example.com/api', '%2e%2e/admin'),
    ).toThrow('Resolved URL must stay within the configured base URL.');
  });
});
