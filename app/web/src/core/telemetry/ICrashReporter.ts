export interface ICrashReporter {
  logBreadcrumb(message: string): void;
  reportError(error: Error, context?: Record<string, unknown>): void;
}
