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
  defaultSortThreshold: 100,
  onSortThresholdChange: vi.fn(),
  trayIconEnabled: false,
  onTrayIconEnabledChange: vi.fn(),
  watchRoot: '/old/root',
  defaultWatchRoot: '/default/root',
  ignorePaths: ['/ignore/a', '/ignore/b'],
  defaultIgnorePaths: ['/default/ignore'],
  onReset: vi.fn(),
  themeResetToken: 0,
  onWatchConfigChange: vi.fn(),
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

  it('resets inputs to defaults before invoking onReset', () => {
    const onReset = vi.fn();
    const onWatchConfigChange = vi.fn();
    const onSortThresholdChange = vi.fn();
    render(
      <PreferencesOverlay
        {...baseProps}
        onReset={onReset}
        onWatchConfigChange={onWatchConfigChange}
        onSortThresholdChange={onSortThresholdChange}
      />,
    );

    fireEvent.click(screen.getByText('preferences.reset'));

    expect(screen.getByLabelText('preferences.sortingLimit.label')).toHaveValue(
      String(baseProps.defaultSortThreshold),
    );
    expect(screen.getByLabelText('watchRoot.label')).toHaveValue(baseProps.defaultWatchRoot);
    expect(screen.getByLabelText('ignorePaths.label')).toHaveValue(
      baseProps.defaultIgnorePaths.join('\n'),
    );
    expect(onReset).toHaveBeenCalledTimes(1);
    expect(onSortThresholdChange).not.toHaveBeenCalled();
    expect(onWatchConfigChange).not.toHaveBeenCalled();
  });
});
