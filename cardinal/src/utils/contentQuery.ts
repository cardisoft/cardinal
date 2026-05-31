const CONTENT_PREFIX = 'content:';

const readQuotedValue = (query: string, start: number): { value: string; end: number } => {
  let value = '';
  let index = start;

  while (index < query.length) {
    const char = query[index];
    if (char === '\\' && index + 1 < query.length) {
      value += query[index + 1];
      index += 2;
      continue;
    }
    if (char === '"') {
      return { value, end: index + 1 };
    }
    value += char;
    index += 1;
  }

  return { value, end: index };
};

const readBareValue = (query: string, start: number): { value: string; end: number } => {
  let index = start;
  while (index < query.length && !/\s/.test(query[index])) {
    index += 1;
  }
  return { value: query.slice(start, index), end: index };
};

export const extractContentTerms = (query: string | null | undefined): string[] => {
  if (!query) {
    return [];
  }

  const terms: string[] = [];
  const seen = new Set<string>();
  const lowerQuery = query.toLocaleLowerCase();
  let index = 0;

  while (index < query.length) {
    const matchIndex = lowerQuery.indexOf(CONTENT_PREFIX, index);
    if (matchIndex === -1) {
      break;
    }

    const valueStart = matchIndex + CONTENT_PREFIX.length;
    const parsed =
      query[valueStart] === '"'
        ? readQuotedValue(query, valueStart + 1)
        : readBareValue(query, valueStart);
    const value = parsed.value.trim();

    if (value && !seen.has(value)) {
      seen.add(value);
      terms.push(value);
    }

    index = Math.max(parsed.end, valueStart + 1);
  }

  return terms;
};
