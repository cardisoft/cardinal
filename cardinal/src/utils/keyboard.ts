type ModifierKeyState = {
  altKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
};

type ModifierKeyOptions = {
  includeAlt?: boolean;
  includeShift?: boolean;
};

export const hasModifierKey = (
  event: ModifierKeyState,
  { includeAlt = true, includeShift = true }: ModifierKeyOptions = {},
): boolean =>
  (includeAlt && event.altKey) ||
  event.ctrlKey ||
  event.metaKey ||
  (includeShift && event.shiftKey);
