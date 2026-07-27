import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { CLOSE_ACTIVE_TAB_EVENT } from '../constants/appEvents';
import { subscribeFinderSearchRequest } from '../runtime/tauriEventRuntime';
import { DEFAULT_SORTABLE_RESULT_THRESHOLD } from '../hooks/useRemoteSort';
import { useAppPreferences } from '../hooks/useAppPreferences';
import { useFullDiskAccessPermission } from '../hooks/useFullDiskAccessPermission';
import { PermissionOverlay } from './PermissionOverlay';
import PreferencesOverlay from './PreferencesOverlay';
import { TabContent, type TabRuntimeControls } from './TabContent';

type WorkspaceTab = {
  id: string;
  title: string;
  initialQuery?: string;
};

const createTab = (number: number, title: string, initialQuery?: string): WorkspaceTab => ({
  id: `tab-${number}`,
  title,
  initialQuery,
});

export function TabManager(): React.JSX.Element {
  const { t, i18n } = useTranslation();
  const nextTabNumberRef = useRef(2);
  const lastShortcutCloseAtRef = useRef(Number.NEGATIVE_INFINITY);
  const controlsByTabRef = useRef(new Map<string, TabRuntimeControls>());
  const activeTabIdRef = useRef('tab-1');
  const [tabs, setTabs] = useState<WorkspaceTab[]>(() => [createTab(1, t('tabs.newSearch'))]);
  const [activeTabId, setActiveTabId] = useState('tab-1');
  const [, bumpControlsVersion] = useState(0);

  activeTabIdRef.current = activeTabId;

  const handleControlsChange = useCallback(
    (tabId: string, controls: TabRuntimeControls | null) => {
      if (controls) {
        controlsByTabRef.current.set(tabId, controls);
      } else {
        controlsByTabRef.current.delete(tabId);
      }
      bumpControlsVersion((version) => version + 1);
    },
    [],
  );

  const refreshActiveSearch = useCallback(() => {
    controlsByTabRef.current.get(activeTabIdRef.current)?.refreshSearchResults();
  }, []);

  const handleTitleChange = useCallback((tabId: string, title: string) => {
    setTabs((currentTabs) => {
      const tab = currentTabs.find((item) => item.id === tabId);
      if (!tab || tab.title === title) {
        return currentTabs;
      }
      return currentTabs.map((item) => (item.id === tabId ? { ...item, title } : item));
    });
  }, []);

  const {
    status: fullDiskAccessStatus,
    isChecking: isCheckingFullDiskAccess,
    requestPermission: requestFullDiskAccessPermission,
  } = useFullDiskAccessPermission();

  const {
    isPreferencesOpen,
    closePreferences,
    trayIconEnabled,
    setTrayIconEnabled,
    watchRoot,
    defaultWatchRoot,
    ignorePaths,
    defaultIgnorePaths,
    includePaths,
    defaultIncludePaths,
    preferencesResetToken,
    handleWatchConfigChange,
    handleResetPreferences,
  } = useAppPreferences({
    fullDiskAccessStatus,
    isCheckingFullDiskAccess,
    refreshSearchResults: refreshActiveSearch,
    i18n,
  });

  const addTab = useCallback((initialQuery?: string) => {
    const title = initialQuery || t('tabs.newSearch');
    const tab = createTab(nextTabNumberRef.current, title, initialQuery);
    nextTabNumberRef.current += 1;
    setTabs((currentTabs) => [...currentTabs, tab]);
    setActiveTabId(tab.id);
  }, [t]);

  useEffect(() => {
    return subscribeFinderSearchRequest(({ query }) => {
      addTab(query);
    });
  }, [addTab]);

  const closeTab = useCallback(
    (tabId: string) => {
      const closeIndex = tabs.findIndex((tab) => tab.id === tabId);
      if (closeIndex < 0) {
        return;
      }

      if (tabs.length === 1) {
        const replacement = createTab(nextTabNumberRef.current, t('tabs.newSearch'));
        nextTabNumberRef.current += 1;
        setTabs([replacement]);
        setActiveTabId(replacement.id);
        return;
      }

      const nextTabs = tabs.filter((tab) => tab.id !== tabId);
      setTabs(nextTabs);
      if (activeTabId === tabId) {
        const nextActiveIndex = Math.min(closeIndex, nextTabs.length - 1);
        setActiveTabId(nextTabs[nextActiveIndex].id);
      }
    },
    [activeTabId, t, tabs],
  );

  const closeActiveTabFromShortcut = useCallback(() => {
    const now = performance.now();
    if (now - lastShortcutCloseAtRef.current < 150) {
      return;
    }
    lastShortcutCloseAtRef.current = now;
    closeTab(activeTabId);
  }, [activeTabId, closeTab]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (!event.metaKey || event.ctrlKey || event.altKey) {
        return;
      }

      const key = event.key.toLowerCase();
      if (key === 't') {
        event.preventDefault();
        addTab();
        return;
      }

      if (key === 'w') {
        event.preventDefault();
        closeActiveTabFromShortcut();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [addTab, closeActiveTabFromShortcut]);

  useEffect(() => {
    window.addEventListener(CLOSE_ACTIVE_TAB_EVENT, closeActiveTabFromShortcut);
    return () => window.removeEventListener(CLOSE_ACTIVE_TAB_EVENT, closeActiveTabFromShortcut);
  }, [closeActiveTabFromShortcut]);

  const showFullDiskAccessOverlay = fullDiskAccessStatus === 'denied';
  const isContentBlocked = showFullDiskAccessOverlay || isPreferencesOpen;
  const activeControls = controlsByTabRef.current.get(activeTabId);
  const overlayStatusMessage = isCheckingFullDiskAccess
    ? t('app.fullDiskAccess.status.checking')
    : t('app.fullDiskAccess.status.disabled');
  const permissionSteps = [
    t('app.fullDiskAccess.steps.one'),
    t('app.fullDiskAccess.steps.two'),
    t('app.fullDiskAccess.steps.three'),
  ];

  return (
    <div className="tab-manager">
      <div className="workspace-tab-bar" role="tablist" aria-label={t('tabs.tabList')}>
        <div className="workspace-tab-list">
          {tabs.map((tab) => {
            const isActive = tab.id === activeTabId;
            return (
              <div
                key={tab.id}
                className={`workspace-tab${isActive ? ' is-active' : ''}`}
              >
                <button
                  id={`workspace-tab-${tab.id}`}
                  type="button"
                  role="tab"
                  aria-selected={isActive}
                  aria-controls={`workspace-tab-panel-${tab.id}`}
                  className="workspace-tab-select"
                  title={tab.title}
                  onClick={() => setActiveTabId(tab.id)}
                >
                  <span className="workspace-tab-title">{tab.title}</span>
                </button>
                <button
                  type="button"
                  className="workspace-tab-close"
                  aria-label={t('tabs.closeTab', { title: tab.title })}
                  title={t('tabs.closeTab', { title: tab.title })}
                  onClick={() => closeTab(tab.id)}
                >
                  ×
                </button>
              </div>
            );
          })}
        </div>
        <button
          type="button"
          className="workspace-tab-add"
          aria-label={t('tabs.newSearch')}
          title={`${t('tabs.newSearch')} (⌘T)`}
          onClick={addTab}
        >
          +
        </button>
      </div>

      <div className="workspace-tab-panels">
        {tabs.map((tab) => {
          const isActive = tab.id === activeTabId;
          return (
            <section
              key={tab.id}
              id={`workspace-tab-panel-${tab.id}`}
              role="tabpanel"
              aria-labelledby={`workspace-tab-${tab.id}`}
              className={`workspace-tab-panel${isActive ? ' is-active' : ''}`}
              hidden={!isActive}
            >
              <TabContent
                tabId={tab.id}
                initialQuery={tab.initialQuery}
                isActive={isActive}
                isBlocked={isContentBlocked}
                onControlsChange={handleControlsChange}
                onTitleChange={handleTitleChange}
              />
            </section>
          );
        })}
      </div>

      <PreferencesOverlay
        open={isPreferencesOpen}
        onClose={closePreferences}
        sortThreshold={activeControls?.sortThreshold ?? DEFAULT_SORTABLE_RESULT_THRESHOLD}
        defaultSortThreshold={DEFAULT_SORTABLE_RESULT_THRESHOLD}
        onSortThresholdChange={(value) => activeControls?.setSortThreshold(value)}
        trayIconEnabled={trayIconEnabled}
        onTrayIconEnabledChange={setTrayIconEnabled}
        watchRoot={watchRoot ?? defaultWatchRoot}
        defaultWatchRoot={defaultWatchRoot}
        onWatchConfigChange={handleWatchConfigChange}
        ignorePaths={ignorePaths}
        defaultIgnorePaths={defaultIgnorePaths}
        includePaths={includePaths}
        defaultIncludePaths={defaultIncludePaths}
        onReset={handleResetPreferences}
        themeResetToken={preferencesResetToken}
      />

      {showFullDiskAccessOverlay && (
        <PermissionOverlay
          title={t('app.fullDiskAccess.title')}
          description={t('app.fullDiskAccess.description')}
          steps={permissionSteps}
          statusMessage={overlayStatusMessage}
          onRequestPermission={requestFullDiskAccessPermission}
          disabled={isCheckingFullDiskAccess}
          actionLabel={t('app.fullDiskAccess.openSettings')}
        />
      )}
    </div>
  );
}
