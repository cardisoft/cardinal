type ModifierKeyState = {
  altKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
};

type ModifierKeyOptions = {
  includeShift?: boolean;
};

export const hasModifierKey = (
  event: ModifierKeyState,
  { includeShift = true }: ModifierKeyOptions = {},
): boolean => event.altKey || event.ctrlKey || event.metaKey || (includeShift && event.shiftKey);
