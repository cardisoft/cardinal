import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { DEFAULT_SHORTCUTS } from '../../shortcuts';
import { ShortcutSettingsOverlay } from '../ShortcutSettingsOverlay';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

describe('ShortcutSettingsOverlay', () => {
  it('records shortcut after selecting an item and saves full shortcut map', async () => {
    const onShortcutSettingsSave = vi.fn().mockResolvedValue(undefined);
    render(
      <ShortcutSettingsOverlay
        open
        onClose={vi.fn()}
        shortcuts={DEFAULT_SHORTCUTS}
        defaultShortcuts={DEFAULT_SHORTCUTS}
        onShortcutSettingsSave={onShortcutSettingsSave}
      />,
    );

    fireEvent.click(screen.getByText('Cmd+Shift+Space'));
    expect(screen.getByText('shortcutSettings.recording')).toBeInTheDocument();

    fireEvent.keyDown(window, { key: 'k', metaKey: true, shiftKey: true });
    expect(screen.getByText('Cmd+Shift+K')).toBeInTheDocument();

    fireEvent.click(screen.getByText('shortcutSettings.save'));

    await waitFor(() => {
      expect(onShortcutSettingsSave).toHaveBeenCalledWith({
        ...DEFAULT_SHORTCUTS,
        quickLaunch: 'Command+Shift+K',
      });
    });
  });

  it('shows capture error for unsupported keys', () => {
    render(
      <ShortcutSettingsOverlay
        open
        onClose={vi.fn()}
        shortcuts={DEFAULT_SHORTCUTS}
        defaultShortcuts={DEFAULT_SHORTCUTS}
        onShortcutSettingsSave={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    fireEvent.click(screen.getByText('Cmd+Shift+Space'));
    fireEvent.keyDown(window, { key: 'Dead' });

    expect(screen.getByText('shortcutSettings.errors.unsupportedKey')).toBeInTheDocument();
  });
});
