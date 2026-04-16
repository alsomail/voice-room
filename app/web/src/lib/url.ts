export function joinConfiguredUrl(base: string, path: string) {
  const trimmedPath = path.trim();

  if (/^[a-zA-Z][a-zA-Z\d+\-.]*:/.test(trimmedPath)) {
    throw new Error('Path must be relative to the configured base URL.');
  }

  if (/(^|\/)\.\.?(\/|$)/.test(trimmedPath)) {
    throw new Error('Path must not contain dot segments.');
  }

  const normalizedBaseUrl = new URL(base.endsWith('/') ? base : `${base}/`);
  const normalizedBasePath = normalizedBaseUrl.pathname.endsWith('/')
    ? normalizedBaseUrl.pathname
    : `${normalizedBaseUrl.pathname}/`;
  const normalizedPath = trimmedPath.replace(/^\/+/, '');
  const resolvedUrl = new URL(normalizedPath, normalizedBaseUrl);

  if (
    resolvedUrl.origin !== normalizedBaseUrl.origin ||
    !resolvedUrl.pathname.startsWith(normalizedBasePath)
  ) {
    throw new Error('Resolved URL must stay within the configured base URL.');
  }

  return resolvedUrl.toString();
}
