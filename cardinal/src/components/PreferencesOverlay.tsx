import React, { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getWatchRootValidation, isPathInputValid } from '../utils/watchRoot';
import type { ServerConfig } from '../hooks/useServerConfig';
import ThemeSwitcher from './ThemeSwitcher';
import LanguageSwitcher from './LanguageSwitcher';

type PreferencesOverlayProps = {
  open: boolean;
  onClose: () => void;
  sortThreshold: number;
  defaultSortThreshold: number;
  onSortThresholdChange: (value: number) => void;
  trayIconEnabled: boolean;
  onTrayIconEnabledChange: (enabled: boolean) => void;
  watchRoot: string;
  defaultWatchRoot: string;
  onPreferencesChange: (next: {
    watchRoot: string;
    ignorePaths: string[];
    includePaths: string[];
    serverConfig: ServerConfig;
  }) => void;
  ignorePaths: string[];
  defaultIgnorePaths: string[];
  includePaths: string[];
  defaultIncludePaths: string[];
  serverConfig: ServerConfig;
  defaultServerConfig: ServerConfig;
  onReset: () => void;
  themeResetToken: number;
};

export function PreferencesOverlay({
  open,
  onClose,
  sortThreshold,
  defaultSortThreshold,
  onSortThresholdChange,
  trayIconEnabled,
  onTrayIconEnabledChange,
  watchRoot,
  defaultWatchRoot,
  onPreferencesChange,
  ignorePaths,
  defaultIgnorePaths,
  includePaths,
  defaultIncludePaths,
  serverConfig,
  defaultServerConfig,
  onReset,
  themeResetToken,
}: PreferencesOverlayProps): React.JSX.Element | null {
  const { t } = useTranslation();
  const [thresholdInput, setThresholdInput] = useState<string>(() => sortThreshold.toString());
  const [watchRootInput, setWatchRootInput] = useState<string>(() => watchRoot);
  const [ignorePathsInput, setIgnorePathsInput] = useState<string>(() => ignorePaths.join('\n'));
  const [includePathsInput, setIncludePathsInput] = useState<string>(() => includePaths.join('\n'));
  const [serverEnabledInput, setServerEnabledInput] = useState<boolean>(() => serverConfig.enabled);
  const [serverEndpointInput, setServerEndpointInput] = useState<string>(
    () => serverConfig.endpoint,
  );

  useEffect(() => {
    if (!open) {
      return;
    }

    const handleKeyDown = (event: KeyboardEvent): void => {
      if (event.key === 'Escape') {
        onClose();
        event.preventDefault();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [open, onClose]);

  useEffect(() => {
    if (!open) {
      return;
    }
    setThresholdInput(sortThreshold.toString());
  }, [open, sortThreshold]);

  useEffect(() => {
    if (!open) {
      return;
    }
    setWatchRootInput(watchRoot);
    setIgnorePathsInput(ignorePaths.join('\n'));
    setIncludePathsInput(includePaths.join('\n'));
    setServerEnabledInput(serverConfig.enabled);
    setServerEndpointInput(serverConfig.endpoint);
  }, [open, watchRoot, ignorePaths, includePaths, serverConfig]);

  const commitThreshold = useCallback(() => {
    const numericText = thresholdInput.replace(/[^\d]/g, '');
    if (!numericText) {
      setThresholdInput(sortThreshold.toString());
      return;
    }
    const parsed = Number.parseInt(numericText, 10);
    if (Number.isNaN(parsed)) {
      setThresholdInput(sortThreshold.toString());
      return;
    }
    const normalized = Math.max(1, Math.round(parsed));
    onSortThresholdChange(normalized);
    setThresholdInput(normalized.toString());
  }, [onSortThresholdChange, sortThreshold, thresholdInput]);

  const handleThresholdChange = (event: React.ChangeEvent<HTMLInputElement>): void => {
    const value = event.target.value;
    if (/^\d*$/.test(value)) {
      setThresholdInput(value);
    }
  };

  const { errorKey: watchRootErrorKey } = getWatchRootValidation(watchRootInput);
  const watchRootErrorMessage = watchRootErrorKey ? t(watchRootErrorKey) : null;

  const parsedIgnorePaths = ignorePathsInput
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
  const ignorePathsErrorMessage = (() => {
    const invalid = parsedIgnorePaths.find((line) => !isPathInputValid(line));
    return invalid ? t('ignorePaths.errors.absolute') : null;
  })();

  const parsedIncludePaths = includePathsInput
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
  const includePathsErrorMessage = (() => {
    const invalid = parsedIncludePaths.find((line) => !isPathInputValid(line));
    return invalid ? t('includePaths.errors.absolute') : null;
  })();

  const trimmedServerEndpoint = serverEndpointInput.trim();
  const serverPortSeparatorIndex = trimmedServerEndpoint.lastIndexOf(':');
  const parsedServerPort =
    serverPortSeparatorIndex >= 0
      ? Number.parseInt(trimmedServerEndpoint.slice(serverPortSeparatorIndex + 1), 10)
      : Number.NaN;
  const serverEndpointErrorMessage =
    trimmedServerEndpoint.length === 0 ||
    serverPortSeparatorIndex <= 0 ||
    serverPortSeparatorIndex === trimmedServerEndpoint.length - 1 ||
    Number.isNaN(parsedServerPort) ||
    parsedServerPort < 1 ||
    parsedServerPort > 65535
      ? t('preferences.server.endpointError', {
          defaultValue: 'Enter an endpoint like 127.0.0.1:3388 or 0.0.0.0:3388.',
        })
      : null;

  const handleSave = (): void => {
    if (
      watchRootErrorMessage ||
      ignorePathsErrorMessage ||
      includePathsErrorMessage ||
      serverEndpointErrorMessage
    ) {
      return;
    }
    commitThreshold();
    const trimmedWatchRoot = watchRootInput.trim();
    onPreferencesChange({
      watchRoot: trimmedWatchRoot,
      ignorePaths: parsedIgnorePaths,
      includePaths: parsedIncludePaths,
      serverConfig: {
        enabled: serverEnabledInput,
        endpoint: trimmedServerEndpoint,
      },
    });
    setWatchRootInput(trimmedWatchRoot);
    setIgnorePathsInput(parsedIgnorePaths.join('\n'));
    setIncludePathsInput(parsedIncludePaths.join('\n'));
    setServerEndpointInput(trimmedServerEndpoint);
    onClose();
  };

  const handleReset = (): void => {
    setThresholdInput(defaultSortThreshold.toString());
    setWatchRootInput(defaultWatchRoot);
    setIgnorePathsInput(defaultIgnorePaths.join('\n'));
    setIncludePathsInput(defaultIncludePaths.join('\n'));
    setServerEnabledInput(defaultServerConfig.enabled);
    setServerEndpointInput(defaultServerConfig.endpoint);
    onReset();
  };

  if (!open) {
    return null;
  }

  const handleOverlayClick = (event: React.MouseEvent<HTMLDivElement>): void => {
    if (event.target === event.currentTarget) {
      onClose();
    }
  };

  return (
    <div
      className="preferences-overlay"
      role="dialog"
      aria-modal="true"
      onClick={handleOverlayClick}
    >
      <div className="preferences-card">
        <header className="preferences-card__header">
          <h1 className="preferences-card__title">{t('preferences.title')}</h1>
        </header>

        <div className="preferences-section">
          <div className="preferences-row">
            <p className="preferences-label">{t('preferences.appearance')}</p>
            <ThemeSwitcher className="preferences-control" resetToken={themeResetToken} />
          </div>
          <div className="preferences-row">
            <p className="preferences-label">{t('preferences.language')}</p>
            <LanguageSwitcher className="preferences-control" />
          </div>
          <div className="preferences-row">
            <p className="preferences-label">{t('preferences.trayIcon.label')}</p>
            <div className="preferences-control">
              <label className="preferences-switch">
                <input
                  className="preferences-switch__input"
                  type="checkbox"
                  checked={trayIconEnabled}
                  onChange={(event) => onTrayIconEnabledChange(event.target.checked)}
                  aria-label={t('preferences.trayIcon.label')}
                />
                <span className="preferences-switch__track" aria-hidden="true" />
              </label>
            </div>
          </div>
          <div className="preferences-row">
            <p className="preferences-label">
              {t('preferences.server.enabled', {
                defaultValue: 'HTTP search server',
              })}
            </p>
            <div className="preferences-control">
              <label className="preferences-switch">
                <input
                  className="preferences-switch__input"
                  type="checkbox"
                  checked={serverEnabledInput}
                  onChange={(event) => setServerEnabledInput(event.target.checked)}
                  aria-label={t('preferences.server.enabled', {
                    defaultValue: 'HTTP search server',
                  })}
                />
                <span className="preferences-switch__track" aria-hidden="true" />
              </label>
            </div>
          </div>
          <div className="preferences-row">
            <div className="preferences-row__details">
              <p className="preferences-label">
                {t('preferences.server.endpoint', {
                  defaultValue: 'HTTP server endpoint',
                })}
              </p>
            </div>
            <div className="preferences-control">
              <input
                className="preferences-field preferences-number-input preferences-watch-root-input"
                type="text"
                value={serverEndpointInput}
                onChange={(event) => setServerEndpointInput(event.target.value)}
                aria-label={t('preferences.server.endpoint', {
                  defaultValue: 'HTTP server endpoint',
                })}
                autoComplete="off"
                spellCheck={false}
              />
              {serverEndpointErrorMessage ? (
                <p
                  className="permission-status permission-status--error preferences-field-error"
                  role="status"
                  aria-live="polite"
                >
                  {serverEndpointErrorMessage}
                </p>
              ) : null}
            </div>
          </div>
          <div className="preferences-row">
            <div className="preferences-row__details">
              <p className="preferences-label">{t('preferences.sortingLimit.label')}</p>
            </div>
            <div className="preferences-control">
              <input
                className="preferences-field preferences-number-input"
                type="text"
                inputMode="numeric"
                pattern="[0-9]*"
                value={thresholdInput}
                onChange={handleThresholdChange}
                aria-label={t('preferences.sortingLimit.label')}
              />
            </div>
          </div>
          <div className="preferences-row">
            <div className="preferences-row__details">
              <p className="preferences-label" title={t('watchRoot.help')}>
                {t('watchRoot.label')}
              </p>
            </div>
            <div className="preferences-control">
              <input
                className="preferences-field preferences-number-input preferences-watch-root-input"
                type="text"
                value={watchRootInput}
                onChange={(event) => setWatchRootInput(event.target.value)}
                aria-label={t('watchRoot.label')}
                autoComplete="off"
                spellCheck={false}
              />
              {watchRootErrorMessage ? (
                <p
                  className="permission-status permission-status--error preferences-field-error"
                  role="status"
                  aria-live="polite"
                >
                  {watchRootErrorMessage}
                </p>
              ) : null}
            </div>
          </div>
          <div className="preferences-row">
            <div className="preferences-row__details">
              <p className="preferences-label" title={t('ignorePaths.help')}>
                {t('ignorePaths.label')}
              </p>
            </div>
            <div className="preferences-control">
              <textarea
                className="preferences-field preferences-textarea"
                value={ignorePathsInput}
                onChange={(event) => setIgnorePathsInput(event.target.value)}
                aria-label={t('ignorePaths.label')}
                autoComplete="off"
                spellCheck={false}
              />
              {ignorePathsErrorMessage ? (
                <p
                  className="permission-status permission-status--error preferences-field-error"
                  role="status"
                  aria-live="polite"
                >
                  {ignorePathsErrorMessage}
                </p>
              ) : null}
            </div>
          </div>
          <div className="preferences-row">
            <div className="preferences-row__details">
              <p className="preferences-label" title={t('includePaths.help')}>
                {t('includePaths.label')}
              </p>
            </div>
            <div className="preferences-control">
              <textarea
                className="preferences-field preferences-textarea"
                value={includePathsInput}
                onChange={(event) => setIncludePathsInput(event.target.value)}
                aria-label={t('includePaths.label')}
                autoComplete="off"
                spellCheck={false}
              />
              {includePathsErrorMessage ? (
                <p
                  className="permission-status permission-status--error preferences-field-error"
                  role="status"
                  aria-live="polite"
                >
                  {includePathsErrorMessage}
                </p>
              ) : null}
            </div>
          </div>
        </div>
        <footer className="preferences-card__footer">
          <button
            className="preferences-save"
            type="button"
            onClick={handleSave}
            disabled={Boolean(
              watchRootErrorMessage ||
              ignorePathsErrorMessage ||
              includePathsErrorMessage ||
              serverEndpointErrorMessage,
            )}
          >
            {t('preferences.save')}
          </button>
          <button className="preferences-reset" type="button" onClick={handleReset}>
            {t('preferences.reset')}
          </button>
        </footer>
      </div>
    </div>
  );
}

export default PreferencesOverlay;
