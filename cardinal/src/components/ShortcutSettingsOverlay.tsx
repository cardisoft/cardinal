import React, { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { SHORTCUT_DEFINITIONS, type ShortcutId, type ShortcutMap } from '../shortcuts';
import {
  captureShortcutFromKeydown,
  formatShortcutForDisplay,
  type ShortcutCaptureError,
} from '../utils/shortcutCapture';

type ShortcutSettingsOverlayProps = {
  open: boolean;
  onClose: () => void;
  shortcuts: ShortcutMap;
  defaultShortcuts: ShortcutMap;
  onShortcutSettingsSave: (shortcuts: ShortcutMap) => Promise<void>;
};

const CAPTURE_ERROR_KEY_MAP: Record<ShortcutCaptureError, string> = {
  modifierRequired: 'shortcutSettings.errors.modifierRequired',
  keyRequired: 'shortcutSettings.errors.keyRequired',
  unsupportedKey: 'shortcutSettings.errors.unsupportedKey',
};

export function ShortcutSettingsOverlay({
  open,
  onClose,
  shortcuts,
  defaultShortcuts,
  onShortcutSettingsSave,
}: ShortcutSettingsOverlayProps): React.JSX.Element | null {
  const { t } = useTranslation();
  const recordingText = t('shortcutSettings.recording');
  const [draftShortcuts, setDraftShortcuts] = useState<ShortcutMap>(shortcuts);
  const [recordingShortcutId, setRecordingShortcutId] = useState<ShortcutId | null>(null);
  const [captureError, setCaptureError] = useState<ShortcutCaptureError | null>(null);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    if (!open) {
      return;
    }
    setDraftShortcuts(shortcuts);
    setRecordingShortcutId(null);
    setCaptureError(null);
    setSubmitError(null);
  }, [open, shortcuts]);

  useEffect(() => {
    if (!open || !recordingShortcutId) {
      return;
    }

    const handleKeydown = (event: KeyboardEvent): void => {
      event.preventDefault();
      event.stopPropagation();

      const result = captureShortcutFromKeydown(event, false);
      if (result.error) {
        setCaptureError(result.error);
        return;
      }

      setCaptureError(null);
      setSubmitError(null);
      setDraftShortcuts((prev) => ({
        ...prev,
        [recordingShortcutId]: result.shortcut,
      }));
      setRecordingShortcutId(null);
    };

    window.addEventListener('keydown', handleKeydown, true);
    return () => window.removeEventListener('keydown', handleKeydown, true);
  }, [open, recordingShortcutId]);

  const handleOverlayClick = (event: React.MouseEvent<HTMLDivElement>): void => {
    if (event.target === event.currentTarget && !isSaving && !recordingShortcutId) {
      onClose();
    }
  };

  const handleDialogKeyDown = (event: React.KeyboardEvent<HTMLDivElement>): void => {
    if (recordingShortcutId) {
      event.stopPropagation();
      return;
    }

    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      if (!isSaving) {
        onClose();
      }
      return;
    }
    event.stopPropagation();
  };

  const handleReset = (): void => {
    setCaptureError(null);
    setSubmitError(null);
    setRecordingShortcutId(null);
    setDraftShortcuts(defaultShortcuts);
  };

  const handleCaptureButtonClick = useCallback((shortcutId: ShortcutId): void => {
    setCaptureError(null);
    setSubmitError(null);
    setRecordingShortcutId((current) => (current === shortcutId ? null : shortcutId));
  }, []);

  const handleSave = useCallback(async (): Promise<void> => {
    if (isSaving || recordingShortcutId) {
      return;
    }

    setSubmitError(null);
    setIsSaving(true);
    try {
      await onShortcutSettingsSave(draftShortcuts);
      onClose();
    } catch (error) {
      console.error('Failed to update shortcut settings', error);
      setSubmitError(t('shortcutSettings.errors.registerFailed'));
    } finally {
      setIsSaving(false);
    }
  }, [draftShortcuts, isSaving, onClose, onShortcutSettingsSave, recordingShortcutId, t]);

  if (!open) {
    return null;
  }

  return (
    <div
      className="shortcut-settings-overlay"
      role="dialog"
      aria-modal="true"
      aria-label={t('shortcutSettings.title')}
      onClick={handleOverlayClick}
      onKeyDown={handleDialogKeyDown}
    >
      <div className="shortcut-settings-card">
        <header className="shortcut-settings-card__header">
          <h2 className="shortcut-settings-card__title">{t('shortcutSettings.title')}</h2>
        </header>

        <div className="shortcut-settings-section">
          <p className="shortcut-settings-capture-hint">{t('shortcutSettings.captureHint')}</p>
          {SHORTCUT_DEFINITIONS.map((shortcutId) => {
            const labelKey = `shortcutSettings.items.${shortcutId}.label`;
            const descriptionKey = `shortcutSettings.items.${shortcutId}.description`;
            const label = t(labelKey);
            const isRecording = recordingShortcutId === shortcutId;
            const buttonText = isRecording
              ? recordingText
              : formatShortcutForDisplay(draftShortcuts[shortcutId]);

            return (
              <div key={shortcutId} className="shortcut-settings-row">
                <div className="shortcut-settings-row__details">
                  <p className="preferences-label">{label}</p>
                  <p className="shortcut-settings-hint">{t(descriptionKey)}</p>
                </div>
                <div className="preferences-control">
                  <button
                    type="button"
                    className={`shortcut-settings-capture-button${
                      isRecording ? ' shortcut-settings-capture-button--recording' : ''
                    }`}
                    aria-label={`${label}: ${buttonText}`}
                    onClick={() => handleCaptureButtonClick(shortcutId)}
                    disabled={isSaving}
                  >
                    {buttonText}
                  </button>
                </div>
              </div>
            );
          })}
          {captureError ? (
            <p
              className="permission-status permission-status--error shortcut-settings-error"
              role="status"
            >
              {t(CAPTURE_ERROR_KEY_MAP[captureError])}
            </p>
          ) : null}
          {submitError ? (
            <p
              className="permission-status permission-status--error shortcut-settings-error"
              role="status"
            >
              {submitError}
            </p>
          ) : null}
        </div>

        <footer className="shortcut-settings-card__footer">
          <button
            className="preferences-save"
            type="button"
            onClick={() => void handleSave()}
            disabled={isSaving || Boolean(recordingShortcutId)}
          >
            {isSaving ? t('shortcutSettings.saving') : t('shortcutSettings.save')}
          </button>
          <button
            className="preferences-reset"
            type="button"
            onClick={handleReset}
            disabled={isSaving}
          >
            {t('shortcutSettings.reset')}
          </button>
        </footer>
      </div>
    </div>
  );
}

export default ShortcutSettingsOverlay;
