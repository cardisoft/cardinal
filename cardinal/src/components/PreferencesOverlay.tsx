import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getWatchRootValidation, isPathInputValid } from '../utils/watchRoot';
import ThemeSwitcher from './ThemeSwitcher';
import LanguageSwitcher from './LanguageSwitcher';

type PreferencesOverlayProps = {
  open: boolean;
  onClose: () => void;
  sortThreshold: number;
  onSortThresholdChange: (value: number) => void;
  trayIconEnabled: boolean;
  onTrayIconEnabledChange: (enabled: boolean) => void;
  watchRoot: string;
  onWatchConfigChange: (next: { watchRoot: string; ignorePaths: string[] }) => void;
  ignorePaths: string[];
  onReset: () => void;
  themeResetToken: number;
};

export function PreferencesOverlay({
  open,
  onClose,
  sortThreshold,
  onSortThresholdChange,
  trayIconEnabled,
  onTrayIconEnabledChange,
  watchRoot,
  onWatchConfigChange,
  ignorePaths,
  onReset,
  themeResetToken,
}: PreferencesOverlayProps): React.JSX.Element | null {
  const { t } = useTranslation();
  const [thresholdInput, setThresholdInput] = useState<string>(() => sortThreshold.toString());
  const [watchRootInput, setWatchRootInput] = useState<string>(() => watchRoot);
  const [ignorePathsInput, setIgnorePathsInput] = useState<string>(() => ignorePaths.join('\n'));

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
    setWatchRootInput(watchRoot);
    setIgnorePathsInput(ignorePaths.join('\n'));
  }, [open, sortThreshold, watchRoot, ignorePaths]);

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

  const { errorKey: watchRootErrorKey } = useMemo(
    () => getWatchRootValidation(watchRootInput),
    [watchRootInput],
  );
  const watchRootErrorMessage = watchRootErrorKey ? t(watchRootErrorKey) : null;

  const handleWatchRootKeyDown = (event: React.KeyboardEvent<HTMLInputElement>): void => {
    if (event.key === 'Escape') {
      setWatchRootInput(watchRoot);
    }
  };

  const parsedIgnorePaths = useMemo(
    () =>
      ignorePathsInput
        .split(/\r?\n/)
        .map((line) => line.trim())
        .filter((line) => line.length > 0),
    [ignorePathsInput],
  );
  const ignorePathsErrorMessage = useMemo(() => {
    const invalid = parsedIgnorePaths.find((line) => !isPathInputValid(line));
    return invalid ? t('ignorePaths.errors.absolute') : null;
  }, [parsedIgnorePaths, t]);

  const handleIgnorePathsKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>): void => {
    if (event.key === 'Escape') {
      setIgnorePathsInput(ignorePaths.join('\n'));
    }
  };

  const handleSave = (): void => {
    if (watchRootErrorMessage || ignorePathsErrorMessage) {
      return;
    }
    commitThreshold();
    const trimmedWatchRoot = watchRootInput.trim();
    onWatchConfigChange({ watchRoot: trimmedWatchRoot, ignorePaths: parsedIgnorePaths });
    setWatchRootInput(trimmedWatchRoot);
    setIgnorePathsInput(parsedIgnorePaths.join('\n'));
    onClose();
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
                className="preferences-field preferences-number-input"
                type="text"
                value={watchRootInput}
                onChange={(event) => setWatchRootInput(event.target.value)}
                onKeyDown={handleWatchRootKeyDown}
                aria-label={t('watchRoot.label')}
                autoComplete="off"
                spellCheck={false}
                style={{ width: 240, textAlign: 'left' }}
              />
              {watchRootErrorMessage ? (
                <p
                  className="permission-status permission-status--error"
                  role="status"
                  aria-live="polite"
                  style={{ marginBottom: 0 }}
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
                onKeyDown={handleIgnorePathsKeyDown}
                aria-label={t('ignorePaths.label')}
                autoComplete="off"
                spellCheck={false}
              />
              {ignorePathsErrorMessage ? (
                <p
                  className="permission-status permission-status--error"
                  role="status"
                  aria-live="polite"
                  style={{ marginBottom: 0 }}
                >
                  {ignorePathsErrorMessage}
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
            disabled={Boolean(watchRootErrorMessage || ignorePathsErrorMessage)}
          >
            {t('preferences.save')}
          </button>
          <button className="preferences-reset" type="button" onClick={onReset}>
            {t('preferences.reset')}
          </button>
        </footer>
      </div>
    </div>
  );
}

export default PreferencesOverlay;
