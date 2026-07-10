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
  includePaths: ['/include/a'],
  defaultIncludePaths: [] as string[],
  serverConfig: {
    enabled: false,
    endpoint: '127.0.0.1:3388',
  },
  defaultServerConfig: {
    enabled: false,
    endpoint: '127.0.0.1:3388',
  },
  onReset: vi.fn(),
  themeResetToken: 0,
  onPreferencesChange: vi.fn(),
};

describe('PreferencesOverlay', () => {
  it('saves watch root updates via onPreferencesChange', () => {
    const onPreferencesChange = vi.fn();
    render(<PreferencesOverlay {...baseProps} onPreferencesChange={onPreferencesChange} />);

    const watchRootInput = screen.getByLabelText('watchRoot.label');
    fireEvent.change(watchRootInput, { target: { value: '/new/root' } });

    fireEvent.click(screen.getByText('preferences.save'));

    expect(onPreferencesChange).toHaveBeenCalledWith({
      watchRoot: '/new/root',
      ignorePaths: baseProps.ignorePaths,
      includePaths: baseProps.includePaths,
      serverConfig: baseProps.serverConfig,
    });
  });

  it('saves ignore path updates via onPreferencesChange', () => {
    const onPreferencesChange = vi.fn();
    render(<PreferencesOverlay {...baseProps} onPreferencesChange={onPreferencesChange} />);

    const ignorePathsInput = screen.getByLabelText('ignorePaths.label');
    fireEvent.change(ignorePathsInput, { target: { value: '/tmp/one\n/tmp/two' } });

    fireEvent.click(screen.getByText('preferences.save'));

    expect(onPreferencesChange).toHaveBeenCalledWith({
      watchRoot: baseProps.watchRoot,
      ignorePaths: ['/tmp/one', '/tmp/two'],
      includePaths: baseProps.includePaths,
      serverConfig: baseProps.serverConfig,
    });
  });

  it('saves include path updates via onPreferencesChange', () => {
    const onPreferencesChange = vi.fn();
    render(<PreferencesOverlay {...baseProps} onPreferencesChange={onPreferencesChange} />);

    const includePathsInput = screen.getByLabelText('includePaths.label');
    fireEvent.change(includePathsInput, {
      target: { value: '/Volumes/media\n/Volumes/work' },
    });

    fireEvent.click(screen.getByText('preferences.save'));

    expect(onPreferencesChange).toHaveBeenCalledWith({
      watchRoot: baseProps.watchRoot,
      ignorePaths: baseProps.ignorePaths,
      includePaths: ['/Volumes/media', '/Volumes/work'],
      serverConfig: baseProps.serverConfig,
    });
  });

  it('blocks save when an include path is not absolute', () => {
    const onPreferencesChange = vi.fn();
    render(<PreferencesOverlay {...baseProps} onPreferencesChange={onPreferencesChange} />);

    const includePathsInput = screen.getByLabelText('includePaths.label');
    fireEvent.change(includePathsInput, { target: { value: 'relative/path' } });

    const saveButton = screen.getByText('preferences.save') as HTMLButtonElement;
    expect(saveButton.disabled).toBe(true);
    fireEvent.click(saveButton);
    expect(onPreferencesChange).not.toHaveBeenCalled();
  });

  it('saves server config updates via onPreferencesChange', () => {
    const onPreferencesChange = vi.fn();
    render(<PreferencesOverlay {...baseProps} onPreferencesChange={onPreferencesChange} />);

    fireEvent.click(screen.getByLabelText('preferences.server.enabled'));
    fireEvent.change(screen.getByLabelText('preferences.server.endpoint'), {
      target: { value: '0.0.0.0:3390' },
    });
    fireEvent.click(screen.getByText('preferences.save'));

    expect(onPreferencesChange).toHaveBeenCalledWith({
      watchRoot: baseProps.watchRoot,
      ignorePaths: baseProps.ignorePaths,
      includePaths: baseProps.includePaths,
      serverConfig: {
        enabled: true,
        endpoint: '0.0.0.0:3390',
      },
    });
  });

  it('blocks save when server endpoint has a port outside the valid range', () => {
    const onPreferencesChange = vi.fn();
    render(<PreferencesOverlay {...baseProps} onPreferencesChange={onPreferencesChange} />);

    fireEvent.change(screen.getByLabelText('preferences.server.endpoint'), {
      target: { value: '127.0.0.1:70000' },
    });

    const saveButton = screen.getByText('preferences.save') as HTMLButtonElement;
    expect(saveButton.disabled).toBe(true);
    fireEvent.click(saveButton);
    expect(onPreferencesChange).not.toHaveBeenCalled();
  });

  it('blocks save when server endpoint contains a non-digit port suffix', () => {
    const onPreferencesChange = vi.fn();
    render(<PreferencesOverlay {...baseProps} onPreferencesChange={onPreferencesChange} />);

    fireEvent.change(screen.getByLabelText('preferences.server.endpoint'), {
      target: { value: '127.0.0.1:3388abc' },
    });

    const saveButton = screen.getByText('preferences.save') as HTMLButtonElement;
    expect(saveButton.disabled).toBe(true);
    fireEvent.click(saveButton);
    expect(onPreferencesChange).not.toHaveBeenCalled();
  });

  it('resets inputs to defaults before invoking onReset', () => {
    const onReset = vi.fn();
    const onPreferencesChange = vi.fn();
    const onSortThresholdChange = vi.fn();
    render(
      <PreferencesOverlay
        {...baseProps}
        onReset={onReset}
        onPreferencesChange={onPreferencesChange}
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
    expect(screen.getByLabelText('includePaths.label')).toHaveValue(
      baseProps.defaultIncludePaths.join('\n'),
    );
    expect(screen.getByLabelText('preferences.server.enabled')).not.toBeChecked();
    expect(screen.getByLabelText('preferences.server.endpoint')).toHaveValue(
      baseProps.defaultServerConfig.endpoint,
    );
    expect(onReset).toHaveBeenCalledTimes(1);
    expect(onSortThresholdChange).not.toHaveBeenCalled();
    expect(onPreferencesChange).not.toHaveBeenCalled();
  });

  it('applies staged reset values when saved', () => {
    const onPreferencesChange = vi.fn();
    const onSortThresholdChange = vi.fn();
    render(
      <PreferencesOverlay
        {...baseProps}
        onPreferencesChange={onPreferencesChange}
        onSortThresholdChange={onSortThresholdChange}
      />,
    );

    fireEvent.click(screen.getByText('preferences.reset'));
    fireEvent.click(screen.getByText('preferences.save'));

    expect(onSortThresholdChange).toHaveBeenCalledWith(baseProps.defaultSortThreshold);
    expect(onPreferencesChange).toHaveBeenCalledWith({
      watchRoot: baseProps.defaultWatchRoot,
      ignorePaths: baseProps.defaultIgnorePaths,
      includePaths: baseProps.defaultIncludePaths,
      serverConfig: baseProps.defaultServerConfig,
    });
  });

  it('closes preferences on Escape while editing a field', () => {
    const onClose = vi.fn();
    render(<PreferencesOverlay {...baseProps} onClose={onClose} />);

    const includePathsInput = screen.getByLabelText('includePaths.label');
    fireEvent.change(includePathsInput, { target: { value: '/tmp/changed' } });
    fireEvent.keyDown(includePathsInput, { key: 'Escape' });

    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
