import { render, screen, fireEvent } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { PreferencesOverlay } from '../PreferencesOverlay';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock('../ThemeSwitcher', () => ({
  __esModule: true,
  default: () => <div data-testid="theme-switcher" />,
}));

vi.mock('../LanguageSwitcher', () => ({
  __esModule: true,
  default: () => <div data-testid="language-switcher" />,
}));

const baseProps = {
  open: true,
  onClose: vi.fn(),
  sortThreshold: 200,
  onSortThresholdChange: vi.fn(),
  trayIconEnabled: false,
  onTrayIconEnabledChange: vi.fn(),
  watchRoot: '/old/root',
  ignorePaths: ['/ignore/a', '/ignore/b'],
  onReset: vi.fn(),
  themeResetToken: 0,
};

describe('PreferencesOverlay', () => {
  it('saves watch root updates via onWatchConfigChange', () => {
    const onWatchConfigChange = vi.fn();
    render(<PreferencesOverlay {...baseProps} onWatchConfigChange={onWatchConfigChange} />);

    const watchRootInput = screen.getByLabelText('watchRoot.label');
    fireEvent.change(watchRootInput, { target: { value: '/new/root' } });

    fireEvent.click(screen.getByText('preferences.save'));

    expect(onWatchConfigChange).toHaveBeenCalledWith({
      watchRoot: '/new/root',
      ignorePaths: baseProps.ignorePaths,
    });
  });

  it('saves ignore path updates via onWatchConfigChange', () => {
    const onWatchConfigChange = vi.fn();
    render(<PreferencesOverlay {...baseProps} onWatchConfigChange={onWatchConfigChange} />);

    const ignorePathsInput = screen.getByLabelText('ignorePaths.label');
    fireEvent.change(ignorePathsInput, { target: { value: '/tmp/one\n/tmp/two' } });

    fireEvent.click(screen.getByText('preferences.save'));

    expect(onWatchConfigChange).toHaveBeenCalledWith({
      watchRoot: baseProps.watchRoot,
      ignorePaths: ['/tmp/one', '/tmp/two'],
    });
  });
});
