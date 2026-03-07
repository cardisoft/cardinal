const EXTENSION_PREFIX = 'ext:';

type ToolbarReplaceFamily = 'kind' | 'type' | 'date' | 'size' | 'scope';

export type SearchToolbarQueryAction =
  | { kind: 'append-token'; token: string }
  | { kind: 'merge-extension'; extension: string }
  | { kind: 'replace-family'; family: ToolbarReplaceFamily; token: string };

const FAMILY_PREFIXES: Record<ToolbarReplaceFamily, string[]> = {
  kind: ['file:', 'folder:'],
  type: ['type:', 'audio:', 'video:', 'doc:', 'exe:'],
  date: ['dm:', 'datemodified:', 'dc:', 'datecreated:'],
  size: ['size:'],
  scope: ['parent:', 'infolder:', 'in:', 'nosubfolders:'],
};

export const applySearchToolbarQueryAction = (
  query: string,
  action: SearchToolbarQueryAction,
): string => {
  switch (action.kind) {
    case 'append-token':
      return joinTokens([...splitTopLevelTokens(query), action.token]);
    case 'merge-extension':
      return mergeExtensionToken(query, action.extension);
    case 'replace-family':
      return replaceFamilyToken(query, action.family, action.token);
  }
};

const replaceFamilyToken = (query: string, family: ToolbarReplaceFamily, token: string): string => {
  const prefixes = FAMILY_PREFIXES[family];
  const tokens = splitTopLevelTokens(query);

  return replaceTokenFamily(
    tokens,
    (part) => prefixes.some((prefix) => startsWithPrefix(part, prefix)),
    token,
  );
};

const mergeExtensionToken = (query: string, extension: string): string => {
  const normalizedExtension = extension.trim().toLowerCase();
  if (!normalizedExtension) {
    return query.trim();
  }

  const tokens = splitTopLevelTokens(query);
  const existingExtensions = tokens
    .filter((token) => startsWithPrefix(token, EXTENSION_PREFIX))
    .flatMap((token) => parseExtensionToken(token.slice(EXTENSION_PREFIX.length)));

  const mergedExtensions = dedupeCaseInsensitive([...existingExtensions, normalizedExtension]);
  const nextExtensionToken = `${EXTENSION_PREFIX}${mergedExtensions
    .map(formatExtensionValue)
    .join(';')}`;

  return replaceTokenFamily(
    tokens,
    (token) => startsWithPrefix(token, EXTENSION_PREFIX),
    nextExtensionToken,
  );
};

const replaceTokenFamily = (
  tokens: string[],
  shouldReplace: (token: string) => boolean,
  nextToken: string,
): string => {
  const nextTokens: string[] = [];
  let insertedReplacement = false;

  for (const token of tokens) {
    if (!shouldReplace(token)) {
      nextTokens.push(token);
      continue;
    }

    if (!insertedReplacement) {
      nextTokens.push(nextToken);
      insertedReplacement = true;
    }
  }

  if (!insertedReplacement) {
    nextTokens.push(nextToken);
  }

  return joinTokens(nextTokens);
};

const dedupeCaseInsensitive = (values: string[]): string[] => {
  const seen = new Set<string>();
  const deduped: string[] = [];

  for (const value of values) {
    const normalizedValue = value.trim().toLowerCase();
    if (!normalizedValue || seen.has(normalizedValue)) {
      continue;
    }

    seen.add(normalizedValue);
    deduped.push(normalizedValue);
  }

  return deduped;
};

const parseExtensionToken = (rawValue: string): string[] => {
  const values: string[] = [];
  let current = '';
  let inQuote = false;
  let isEscaped = false;

  for (const character of rawValue) {
    if (inQuote) {
      if (isEscaped) {
        current += character;
        isEscaped = false;
        continue;
      }

      if (character === '\\') {
        isEscaped = true;
        continue;
      }

      if (character === '"') {
        inQuote = false;
        continue;
      }

      current += character;
      continue;
    }

    if (character === '"') {
      inQuote = true;
      continue;
    }

    if (character === ';') {
      values.push(current.trim());
      current = '';
      continue;
    }

    current += character;
  }

  values.push(current.trim());
  return values.filter(Boolean);
};

const formatExtensionValue = (value: string): string => {
  if (/^[a-z0-9._+-]+$/i.test(value)) {
    return value;
  }

  const escapedValue = value.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
  return `"${escapedValue}"`;
};

const startsWithPrefix = (token: string, prefix: string): boolean =>
  token.toLowerCase().startsWith(prefix);

const joinTokens = (tokens: string[]): string =>
  tokens
    .map((token) => token.trim())
    .filter(Boolean)
    .join(' ');

const isWhitespace = (value: string): boolean => /\s/.test(value);

const isAngleGroupStart = (query: string, index: number, tokenStart: number): boolean => {
  if (index !== tokenStart) {
    const previousCharacter = query[index - 1];
    if (!isWhitespace(previousCharacter) && !['(', '|', '<'].includes(previousCharacter)) {
      return false;
    }
  }

  const nextCharacter = query[index + 1];
  return nextCharacter !== undefined && !isWhitespace(nextCharacter) && nextCharacter !== '=';
};

export const splitTopLevelTokens = (query: string): string[] => {
  const tokens: string[] = [];
  let tokenStart = -1;
  let parenDepth = 0;
  let angleDepth = 0;
  let inQuote = false;
  let isEscaped = false;

  for (let index = 0; index < query.length; index += 1) {
    const character = query[index];

    if (tokenStart === -1) {
      if (isWhitespace(character)) {
        continue;
      }

      tokenStart = index;
    }

    if (inQuote) {
      if (isEscaped) {
        isEscaped = false;
        continue;
      }

      if (character === '\\') {
        isEscaped = true;
        continue;
      }

      if (character === '"') {
        inQuote = false;
      }

      continue;
    }

    if (character === '"') {
      inQuote = true;
      continue;
    }

    if (character === '(') {
      parenDepth += 1;
      continue;
    }

    if (character === ')' && parenDepth > 0) {
      parenDepth -= 1;
      continue;
    }

    if (character === '<' && isAngleGroupStart(query, index, tokenStart)) {
      angleDepth += 1;
      continue;
    }

    if (character === '>' && angleDepth > 0) {
      angleDepth -= 1;
      continue;
    }

    if (parenDepth === 0 && angleDepth === 0 && isWhitespace(character)) {
      tokens.push(query.slice(tokenStart, index));
      tokenStart = -1;
    }
  }

  if (tokenStart !== -1) {
    tokens.push(query.slice(tokenStart));
  }

  return tokens.filter(Boolean);
};
