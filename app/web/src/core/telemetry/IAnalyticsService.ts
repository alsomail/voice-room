export type AnalyticsPayload = Record<string, unknown>;

export interface IAnalyticsService {
  trackEvent(eventName: string, payload: AnalyticsPayload): void;
  setUserProperties(props: AnalyticsPayload): void;
}
