import { describe, expect, it } from 'vitest';
import {
  captureShortcutFromKeydown,
  formatShortcutForDisplay,
  normalizeShortcut,
  shortcutMatchesKeydown,
  toMenuAccelerator,
} from '../shortcutCapture';

describe('captureShortcutFromKeydown', () => {
  it('captures modifier + key combinations', () => {
    const result = captureShortcutFromKeydown({
      key: 'f',
      metaKey: true,
      ctrlKey: false,
      altKey: false,
      shiftKey: true,
    });

    expect(result).toEqual({
      shortcut: 'Command+Shift+F',
      error: null,
    });
  });

  it('requires at least one modifier', () => {
    const result = captureShortcutFromKeydown({
      key: 'f',
      metaKey: false,
      ctrlKey: false,
      altKey: false,
      shiftKey: false,
    });

    expect(result).toEqual({
      shortcut: null,
      error: 'modifierRequired',
    });
  });

  it('requires a non-modifier key', () => {
    const result = captureShortcutFromKeydown({
      key: 'Shift',
      metaKey: false,
      ctrlKey: false,
      altKey: false,
      shiftKey: true,
    });

    expect(result).toEqual({
      shortcut: null,
      error: 'keyRequired',
    });
  });

  it('maps special keys', () => {
    const result = captureShortcutFromKeydown({
      key: 'ArrowDown',
      metaKey: false,
      ctrlKey: true,
      altKey: true,
      shiftKey: false,
    });

    expect(result).toEqual({
      shortcut: 'Control+Option+Down',
      error: null,
    });
  });

  it('matches comma shortcuts and formats labels', () => {
    expect(
      shortcutMatchesKeydown(
        { key: ',', metaKey: true, ctrlKey: false, altKey: false, shiftKey: false },
        'Command+Comma',
      ),
    ).toBe(true);

    expect(formatShortcutForDisplay('Command+Comma')).toBe('Cmd+,');
    expect(toMenuAccelerator('Command+Comma')).toBe('Cmd+,');
    expect(normalizeShortcut(' cmd + shift + arrowdown ')).toBe('Command+Shift+Down');
  });
});
