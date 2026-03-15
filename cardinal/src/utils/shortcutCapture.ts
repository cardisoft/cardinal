export type ShortcutCaptureError = 'modifierRequired' | 'keyRequired' | 'unsupportedKey';

export type ShortcutCaptureResult =
  | {
      shortcut: string;
      error: null;
    }
  | {
      shortcut: null;
      error: ShortcutCaptureError;
    };

export type ShortcutLikeEvent = Pick<
  KeyboardEvent,
  'key' | 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey'
>;

type ModifierToken = 'Command' | 'Control' | 'Option' | 'Shift';

type ParsedShortcut = {
  modifiers: ModifierToken[];
  key: string;
};

type TokenMetadata = {
  aliases?: readonly string[];
  display?: string;
  menu?: string;
};

const MODIFIER_ORDER: ModifierToken[] = ['Command', 'Control', 'Option', 'Shift'];

const symbolToken = (symbol: string): TokenMetadata => ({
  aliases: [symbol],
  display: symbol,
  menu: symbol,
});

const MODIFIER_METADATA: Record<ModifierToken, TokenMetadata> = {
  Command: {
    aliases: ['cmd', 'meta'],
    display: 'Cmd',
    menu: 'Cmd',
  },
  Control: {
    aliases: ['ctrl'],
    display: 'Ctrl',
    menu: 'Ctrl',
  },
  Option: {
    aliases: ['opt', 'alt'],
    display: 'Opt',
    menu: 'Alt',
  },
  Shift: {
    menu: 'Shift',
  },
};

const SPECIAL_KEY_METADATA: Record<string, TokenMetadata> = {
  Space: {
    aliases: [' ', 'spacebar'],
    menu: 'Space',
  },
  Up: {
    aliases: ['arrowup'],
    display: '↑',
    menu: 'Up',
  },
  Down: {
    aliases: ['arrowdown'],
    display: '↓',
    menu: 'Down',
  },
  Left: {
    aliases: ['arrowleft'],
    display: '←',
    menu: 'Left',
  },
  Right: {
    aliases: ['arrowright'],
    display: '→',
    menu: 'Right',
  },
  Esc: {
    aliases: ['escape'],
    menu: 'Esc',
  },
  Comma: symbolToken(','),
  Period: symbolToken('.'),
  Semicolon: symbolToken(';'),
  Slash: symbolToken('/'),
  Backslash: symbolToken('\\'),
  Backquote: symbolToken('`'),
  BracketLeft: symbolToken('['),
  BracketRight: symbolToken(']'),
  Quote: symbolToken("'"),
  Minus: symbolToken('-'),
  Equal: symbolToken('='),
  Backspace: {},
  Delete: {},
  Insert: {},
  Home: {},
  End: {},
  PageUp: {},
  PageDown: {},
  Tab: {},
  Enter: {},
};

const buildTokenMaps = <T extends string>(metadata: Record<T, TokenMetadata>) => {
  const alias = {} as Record<string, T>;
  const display: Record<string, string> = {};
  const menu: Record<string, string> = {};

  for (const [token, value] of Object.entries(metadata) as Array<[T, TokenMetadata]>) {
    alias[token.toLowerCase()] = token;
    for (const tokenAlias of value.aliases ?? []) {
      alias[tokenAlias.toLowerCase()] = token;
    }
    if (value.display) {
      display[token] = value.display;
    }
    if (value.menu) {
      menu[token] = value.menu;
    }
  }

  return { alias, display, menu };
};

const modifierMaps = buildTokenMaps(MODIFIER_METADATA);
const keyMaps = buildTokenMaps(SPECIAL_KEY_METADATA);

const MODIFIER_ALIAS = modifierMaps.alias;
const KEY_ALIAS = keyMaps.alias;

const DISPLAY_TOKEN_MAP: Record<string, string> = {
  ...modifierMaps.display,
  ...keyMaps.display,
};

const MENU_TOKEN_MAP: Record<string, string> = {
  ...modifierMaps.menu,
  ...keyMaps.menu,
};

const isModifierOnlyKey = (key: string): boolean =>
  key === 'Meta' || key === 'Control' || key === 'Alt' || key === 'Shift';

const normalizeModifierToken = (token: string): ModifierToken | null => {
  const normalized = MODIFIER_ALIAS[token.trim().toLowerCase()];
  return normalized ?? null;
};

const normalizeKeyToken = (token: string): string | null => {
  const normalized = token === ' ' ? token : token.trim();
  if (!normalized) {
    return null;
  }

  if (/^[a-z]$/i.test(normalized)) {
    return normalized.toUpperCase();
  }
  if (/^[0-9]$/.test(normalized)) {
    return normalized;
  }
  if (/^f([1-9]|1[0-9]|2[0-4])$/i.test(normalized)) {
    return normalized.toUpperCase();
  }

  const aliased = KEY_ALIAS[normalized.toLowerCase()];
  return aliased ?? null;
};

const parseShortcut = (shortcut: string): ParsedShortcut | null => {
  const tokens = shortcut
    .split('+')
    .map((part) => part.trim())
    .filter((part) => part.length > 0);

  if (!tokens.length) {
    return null;
  }

  const modifiers: ModifierToken[] = [];
  let key: string | null = null;

  for (const token of tokens) {
    const modifier = normalizeModifierToken(token);
    if (modifier) {
      if (!modifiers.includes(modifier)) {
        modifiers.push(modifier);
      }
      continue;
    }

    const normalizedKey = normalizeKeyToken(token);
    if (!normalizedKey || key) {
      return null;
    }
    key = normalizedKey;
  }

  if (!key) {
    return null;
  }

  const orderedModifiers = MODIFIER_ORDER.filter((modifier) => modifiers.includes(modifier));
  return {
    modifiers: orderedModifiers,
    key,
  };
};

const serializeShortcut = ({ modifiers, key }: ParsedShortcut): string =>
  [...modifiers, key].join('+');

const getModifiersFromEvent = (event: ShortcutLikeEvent): ModifierToken[] => {
  const modifiers: ModifierToken[] = [];
  if (event.metaKey) {
    modifiers.push('Command');
  }
  if (event.ctrlKey) {
    modifiers.push('Control');
  }
  if (event.altKey) {
    modifiers.push('Option');
  }
  if (event.shiftKey) {
    modifiers.push('Shift');
  }
  return modifiers;
};

export const normalizeShortcut = (shortcut: string): string | null => {
  const parsed = parseShortcut(shortcut);
  return parsed ? serializeShortcut(parsed) : null;
};

export const captureShortcutFromKeydown = (
  event: ShortcutLikeEvent,
  requireModifier = true,
): ShortcutCaptureResult => {
  const modifiers = getModifiersFromEvent(event);

  if (isModifierOnlyKey(event.key)) {
    return { shortcut: null, error: 'keyRequired' };
  }

  const key = normalizeKeyToken(event.key);
  if (!key) {
    return { shortcut: null, error: 'unsupportedKey' };
  }

  if (requireModifier && modifiers.length === 0) {
    return { shortcut: null, error: 'modifierRequired' };
  }

  return {
    shortcut: serializeShortcut({ modifiers, key }),
    error: null,
  };
};

export const shortcutMatchesKeydown = (event: ShortcutLikeEvent, shortcut: string): boolean => {
  const expected = normalizeShortcut(shortcut);
  if (!expected) {
    return false;
  }

  const captured = captureShortcutFromKeydown(event, false);
  if (captured.error) {
    return false;
  }

  return captured.shortcut === expected;
};

export const formatShortcutForDisplay = (shortcut: string): string => {
  const normalized = normalizeShortcut(shortcut);
  if (!normalized) {
    return shortcut;
  }

  return normalized
    .split('+')
    .map((token) => DISPLAY_TOKEN_MAP[token] ?? token)
    .join('+');
};

export const toMenuAccelerator = (shortcut: string): string | undefined => {
  const parsed = parseShortcut(shortcut);
  if (!parsed) {
    return undefined;
  }

  const mappedModifiers = parsed.modifiers.map((modifier) => MENU_TOKEN_MAP[modifier] ?? modifier);
  const mappedKey = MENU_TOKEN_MAP[parsed.key] ?? parsed.key;
  return [...mappedModifiers, mappedKey].join('+');
};
