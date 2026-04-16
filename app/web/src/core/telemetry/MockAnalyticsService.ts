import type { AnalyticsPayload, IAnalyticsService } from './IAnalyticsService';
import {
  createTelemetryBaseContext,
  type TelemetryBaseContext,
} from './telemetryContext';

export interface AnalyticsEnvelope {
  eventName: string;
  payload: AnalyticsPayload;
}

interface MockAnalyticsServiceOptions {
  sink?: (event: AnalyticsEnvelope) => void;
  getBaseContext?: () => TelemetryBaseContext;
}

export class MockAnalyticsService implements IAnalyticsService {
  private readonly sink: (event: AnalyticsEnvelope) => void;
  private readonly getBaseContext: () => TelemetryBaseContext;

  constructor(options: MockAnalyticsServiceOptions = {}) {
    this.sink =
      options.sink ?? ((event) => console.info('[mock-analytics]', event));
    this.getBaseContext = options.getBaseContext ?? createTelemetryBaseContext;
  }

  trackEvent(eventName: string, payload: AnalyticsPayload): void {
    this.sink({
      eventName,
      payload: {
        ...payload,
        ...this.getBaseContext(),
      },
    });
  }

  setUserProperties(props: AnalyticsPayload): void {
    this.sink({
      eventName: 'user_properties_updated',
      payload: {
        ...props,
        ...this.getBaseContext(),
      },
    });
  }
}
