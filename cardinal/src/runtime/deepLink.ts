const CARDINAL_SCHEME = 'cardinal:';
const SEARCH_HOST = 'search';

export type DeepLinkSearchAction = {
  query: string;
  directoryQuery: string;
  directoryScopeOpen: boolean;
};

export const parseDeepLinkSearchUrl = (rawUrl: string): DeepLinkSearchAction | null => {
  let url: URL;
  try {
    url = new URL(rawUrl);
  } catch {
    return null;
  }

  if (url.protocol !== CARDINAL_SCHEME || url.hostname !== SEARCH_HOST) {
    return null;
  }

  // Public URL form: cardinal://search?scope=/path/to/folder&q=keyword
  const query = (url.searchParams.get('q') ?? '').trim();
  const directoryQuery = (url.searchParams.get('scope') ?? '').trim();
  const directoryScopeOpen = directoryQuery.length > 0;

  if (!query && !directoryScopeOpen) {
    return null;
  }

  return { query, directoryQuery, directoryScopeOpen };
};

export const parseDeepLinkSearchUrls = (
  urls: readonly string[] | null | undefined,
): DeepLinkSearchAction[] => {
  if (!urls) {
    return [];
  }
  return urls.flatMap((url) => {
    const payload = parseDeepLinkSearchUrl(url);
    return payload ? [payload] : [];
  });
};
