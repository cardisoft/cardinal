import React, { useCallback, useRef, useLayoutEffect, useState } from 'react';
import type { CSSProperties } from 'react';
import type { AppLifecycleStatus } from '../types/ipc';
import { useTranslation } from 'react-i18next';
import { OPEN_PREFERENCES_EVENT } from '../constants/appEvents';

export type StatusTabKey = 'files' | 'events';

type StatusBarProps = {
  scannedFiles: number;
  processedEvents: number;
  lifecycleState: AppLifecycleStatus;
  searchDurationMs: number | null;
  resultCount: number | null;
  activeTab: StatusTabKey;
  onTabChange: (tab: StatusTabKey) => void;
  onRequestRescan: () => void;
  rescanErrorCount: number;
};

const TABS: StatusTabKey[] = ['files', 'events'];

const LIFECYCLE_META: Record<AppLifecycleStatus, { icon: string; tone: string }> = {
  Initializing: { icon: '○', tone: 'initializing' },
  Updating: { icon: '◑', tone: 'updating' },
  Ready: { icon: '●', tone: 'ready' },
};

const StatusBar = ({
  scannedFiles,
  processedEvents,
  lifecycleState,
  searchDurationMs,
  resultCount,
  activeTab,
  onTabChange,
  onRequestRescan,
  rescanErrorCount,
}: StatusBarProps): React.JSX.Element => {
  const { t } = useTranslation();
  const tabsRef = useRef<HTMLDivElement | null>(null);
  const filesTabRef = useRef<HTMLButtonElement | null>(null);
  const eventsTabRef = useRef<HTMLButtonElement | null>(null);
  const [sliderStyle, setSliderStyle] = useState<CSSProperties>({});

  useLayoutEffect(() => {
    // Keep the active-tab underline aligned even when labels resize or counters update.
    const updateSliderPosition = () => {
      const activeTabRef = activeTab === 'files' ? filesTabRef : eventsTabRef;
      if (activeTabRef.current && tabsRef.current) {
        const tabRect = activeTabRef.current.getBoundingClientRect();
        const containerRect = tabsRef.current.getBoundingClientRect();

        setSliderStyle({
          left: `${tabRect.left - containerRect.left}px`,
          width: `${tabRect.width}px`,
        });
      }
    };

    updateSliderPosition();
    // Re-align the slider when the viewport changes width.
    window.addEventListener('resize', updateSliderPosition);
    return () => window.removeEventListener('resize', updateSliderPosition);
  }, [activeTab, scannedFiles, processedEvents]);

  const handleSelect = useCallback(
    (tabKey: StatusTabKey) => {
      if (tabKey === activeTab) return;
      onTabChange(tabKey);
    },
    [activeTab, onTabChange],
  );

  const resultsText =
    typeof resultCount === 'number'
      ? t('statusBar.resultsCount', {
          count: resultCount,
          formatted: resultCount.toLocaleString(),
        })
      : t('statusBar.resultsUnavailable');
  const formattedDuration =
    typeof searchDurationMs === 'number' ? Math.round(searchDurationMs).toLocaleString() : null;
  const durationText =
    formattedDuration != null ? t('statusBar.duration', { value: formattedDuration }) : null;
  const searchDisplay = durationText
    ? t('statusBar.resultsWithDuration', { results: resultsText, duration: durationText })
    : resultsText;
  const lifecycleMeta = LIFECYCLE_META[lifecycleState];
  const lifecycleLabel = t(`statusBar.lifecycle.${lifecycleState}`);
  const rescanDisabled = lifecycleState === 'Initializing';
  const rescanTitle = rescanDisabled
    ? t('statusBar.rescan.disabledHint')
    : t('statusBar.rescan.enabledHint');
  const rescanTooltip =
    rescanErrorCount > 0
      ? `${rescanTitle}\n${t('statusBar.rescan.errorCountSuffix', {
          count: rescanErrorCount,
          formatted: rescanErrorCount.toLocaleString(),
        })}`
      : rescanTitle;
  const indicatorLabel = t('statusBar.aria.status', { status: lifecycleLabel });

  const handleRescanClick = useCallback(() => {
    if (rescanDisabled) {
      return;
    }
    onRequestRescan();
  }, [onRequestRescan, rescanDisabled]);

  const handleOpenPreferences = useCallback(() => {
    const event = new Event(OPEN_PREFERENCES_EVENT);
    window.dispatchEvent(event);
  }, []);

  return (
    <div className="status-bar">
      <div className="status-left">
        <div className="status-section">
          <span
            className={`readiness-indicator ${lifecycleMeta.tone}`}
            aria-label={indicatorLabel}
            title={indicatorLabel}
          >
            {lifecycleMeta.icon}
          </span>
          <span className="status-text">{lifecycleLabel}</span>
        </div>
        <div
          ref={tabsRef}
          className="status-tabs"
          role="tablist"
          aria-label={t('statusBar.aria.tablist')}
        >
          <div className="status-tabs-slider" style={sliderStyle} />
          {TABS.map((key) => {
            const isActive = activeTab === key;
            const label = t(`statusBar.tabs.${key}`);
            const value =
              key === 'files' ? scannedFiles.toLocaleString() : processedEvents.toLocaleString();
            const ref = key === 'files' ? filesTabRef : eventsTabRef;
            return (
              <button
                key={key}
                ref={ref}
                type="button"
                role="tab"
                aria-selected={isActive}
                className={`status-tab ${isActive ? 'is-active' : ''}`}
                data-tone={key}
                onClick={() => handleSelect(key)}
              >
                <span className="status-tab__label">{label}</span>
                <span className="status-tab__value">{value}</span>
              </button>
            );
          })}
        </div>
        <div className="status-controls">
          <button
            type="button"
            className="status-icon-button status-rescan-button"
            onClick={handleRescanClick}
            disabled={rescanDisabled}
            title={rescanTooltip}
            aria-label={t('statusBar.aria.rescan')}
          >
            <span className="status-rescan-icon" aria-hidden="true">
              ↻
            </span>
            <span className="sr-only">{t('statusBar.aria.rescan')}</span>
          </button>
          <button
            type="button"
            className="status-icon-button status-settings-button"
            onClick={handleOpenPreferences}
            title={t('statusBar.aria.settings')}
            aria-label={t('statusBar.aria.settings')}
          >
            <svg
              className="status-icon-svg"
              viewBox="0 0 24 24"
              role="img"
              aria-hidden="true"
              focusable="false"
            >
              <path
                fill="currentColor"
                d="M19.14 12.936c.036-.304.06-.612.06-.936s-.024-.632-.07-.936l2.03-1.578a.5.5 0 0 0 .12-.642l-1.922-3.323a.5.5 0 0 0-.605-.216l-2.39.96a7.02 7.02 0 0 0-1.62-.936l-.36-2.54a.5.5 0 0 0-.496-.425h-3.844a.5.5 0 0 0-.497.425l-.36 2.54a7.02 7.02 0 0 0-1.62.936l-2.39-.96a.5.5 0 0 0-.605.216L2.74 8.844a.5.5 0 0 0 .12.642l2.03 1.578a7.994 7.994 0 0 0 0 1.872l-2.03 1.578a.5.5 0 0 0-.12.642l1.922 3.323a.5.5 0 0 0 .605.216l2.39-.96c.5.383 1.043.699 1.62.936l.36 2.54a.5.5 0 0 0 .497.425h3.844a.5.5 0 0 0 .496-.425l.36-2.54a7.02 7.02 0 0 0 1.62-.936l2.39.96a.5.5 0 0 0 .605-.216l1.922-3.323a.5.5 0 0 0-.12-.642l-2.03-1.578ZM12 15.5A3.5 3.5 0 1 1 12 8.5a3.5 3.5 0 0 1 0 7Z"
              />
            </svg>
            <span className="sr-only">{t('statusBar.aria.settings')}</span>
          </button>
        </div>
      </div>

      <div className="status-right">
        <div className="status-section">
          <span className="status-label">{t('statusBar.searchLabel')}</span>
          <span className="status-value" title={t('statusBar.resultsTitle')}>
            {searchDisplay}
          </span>
        </div>
      </div>
    </div>
  );
};

export default StatusBar;
