import React, { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import ThemeSwitcher from './ThemeSwitcher';
import LanguageSwitcher from './LanguageSwitcher';

type PreferencesOverlayProps = {
  open: boolean;
  onClose: () => void;
  sortThreshold: number;
  onSortThresholdChange: (value: number) => void;
  trayIconEnabled: boolean;
  onTrayIconEnabledChange: (enabled: boolean) => void;
};

export function PreferencesOverlay({
  open,
  onClose,
  sortThreshold,
  onSortThresholdChange,
  trayIconEnabled,
  onTrayIconEnabledChange,
}: PreferencesOverlayProps): React.JSX.Element | null {
  const { t } = useTranslation();
  const [thresholdInput, setThresholdInput] = useState<string>(() => sortThreshold.toString());

  useEffect(() => {
    if (!open) {
      return;
    }

    const handleKeyDown = (event: KeyboardEvent): void => {
      if (event.key === 'Escape') {
        onClose();
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

  const handleThresholdBlur = (): void => {
    commitThreshold();
  };

  const handleThresholdKeyDown = (event: React.KeyboardEvent<HTMLInputElement>): void => {
    if (event.key === 'Enter') {
      event.preventDefault();
      commitThreshold();
    }
    if (event.key === 'Escape') {
      setThresholdInput(sortThreshold.toString());
    }
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
            <ThemeSwitcher className="preferences-control" />
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
            <div className="preferences-control preferences-control--column">
              <input
                className="preferences-number-input"
                type="text"
                inputMode="numeric"
                pattern="[0-9]*"
                value={thresholdInput}
                onChange={handleThresholdChange}
                onBlur={handleThresholdBlur}
                onKeyDown={handleThresholdKeyDown}
                aria-label={t('preferences.sortingLimit.label')}
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default PreferencesOverlay;
