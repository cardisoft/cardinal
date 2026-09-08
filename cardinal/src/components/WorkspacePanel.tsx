import { useState } from 'react';
import type { AppLifecycleStatus } from '../types/ipc';
import type { FavoritePath, SavedSearch } from '../hooks/useWorkspaceCollections';
import { useTranslation } from 'react-i18next';
import { splitPath } from '../utils/path';
import type { FileOperationUpdate } from '../hooks/useContextMenu';

export type WorkspaceSection = 'saved' | 'favorites' | 'coverage' | 'operations';

type CoverageInfo = {
  watchRoot: string;
  ignorePaths: string[];
  includePaths: string[];
  fullDiskAccessGranted: boolean;
  lifecycleState: AppLifecycleStatus;
  scannedFiles: number;
  currentQuery: string;
  currentDirectoryQuery: string;
};

type OperationInfo = {
  indexingPaused: boolean;
  lifecycleState: AppLifecycleStatus;
  activity: 'idle' | 'low' | 'medium' | 'high';
  scannedFiles: number;
  fileOperations: FileOperationUpdate[];
  isSearching: boolean;
  isSorting: boolean;
  onToggleIndexing: () => void;
  onCancelSearch: () => void;
  onCancelSort: () => void;
  onRequestRescan: () => void;
};

type WorkspacePanelProps = {
  activeSection: WorkspaceSection;
  onClose: () => void;
  savedSearches: SavedSearch[];
  favorites: FavoritePath[];
  selectedPaths: string[];
  onSaveSearch: (name: string) => void;
  onApplySavedSearch: (savedSearch: SavedSearch) => void;
  onRemoveSavedSearch: (id: string) => void;
  onAddSelectedFavorites: () => void;
  onOpenFavorite: (path: string) => void;
  onRevealFavorite: (path: string) => void;
  onRemoveFavorite: (path: string) => void;
  coverage: CoverageInfo;
  operations: OperationInfo;
  onOpenPreferences: () => void;
};

const SECTION_KEYS: WorkspaceSection[] = ['saved', 'favorites', 'coverage', 'operations'];

export function WorkspaceToolbar({
  activeSection,
  onSelect,
}: {
  activeSection: WorkspaceSection | null;
  onSelect: (section: WorkspaceSection) => void;
}) {
  const { t } = useTranslation();
  return (
    <nav className="workspace-toolbar" aria-label={t('workspace.toolbarLabel')}>
      {SECTION_KEYS.map((section) => (
        <button
          key={section}
          type="button"
          className={activeSection === section ? 'is-active' : ''}
          aria-pressed={activeSection === section}
          onClick={() => onSelect(section)}
        >
          <span className="workspace-toolbar__icon" aria-hidden="true">
            {section === 'saved'
              ? '⌕'
              : section === 'favorites'
                ? '★'
                : section === 'coverage'
                  ? '?'
                  : '◷'}
          </span>
          {t(`workspace.sections.${section}`)}
        </button>
      ))}
    </nav>
  );
}

export function WorkspacePanel(props: WorkspacePanelProps): React.JSX.Element {
  const { t } = useTranslation();
  const [savedSearchName, setSavedSearchName] = useState('');
  const { activeSection, coverage, operations } = props;

  const saveCurrentSearch = () => {
    const name = savedSearchName.trim();
    if (!name) return;
    props.onSaveSearch(name);
    setSavedSearchName('');
  };

  return (
    <aside className="workspace-panel" aria-label={t(`workspace.sections.${activeSection}`)}>
      <header className="workspace-panel__header">
        <h2>{t(`workspace.sections.${activeSection}`)}</h2>
        <button type="button" aria-label={t('workspace.close')} onClick={props.onClose}>
          ×
        </button>
      </header>

      <div className="workspace-panel__body">
        {activeSection === 'saved' ? (
          <>
            <div className="workspace-inline-form">
              <label htmlFor="saved-search-name">{t('workspace.saved.nameLabel')}</label>
              <div>
                <input
                  id="saved-search-name"
                  value={savedSearchName}
                  onChange={(event) => setSavedSearchName(event.target.value)}
                  onKeyDown={(event) => event.key === 'Enter' && saveCurrentSearch()}
                />
                <button
                  type="button"
                  onClick={saveCurrentSearch}
                  disabled={!savedSearchName.trim()}
                >
                  {t('workspace.saved.saveCurrent')}
                </button>
              </div>
            </div>
            <WorkspaceEmpty
              visible={props.savedSearches.length === 0}
              text={t('workspace.saved.empty')}
            />
            <ul className="workspace-list">
              {props.savedSearches.map((savedSearch) => (
                <li key={savedSearch.id}>
                  <button
                    type="button"
                    className="workspace-list__primary"
                    onClick={() => props.onApplySavedSearch(savedSearch)}
                  >
                    <strong>{savedSearch.name}</strong>
                    <span>{savedSearch.query || t('workspace.saved.allFiles')}</span>
                  </button>
                  <button
                    type="button"
                    aria-label={t('workspace.remove')}
                    onClick={() => props.onRemoveSavedSearch(savedSearch.id)}
                  >
                    ×
                  </button>
                </li>
              ))}
            </ul>
          </>
        ) : null}

        {activeSection === 'favorites' ? (
          <>
            <button
              type="button"
              className="workspace-primary-action"
              onClick={props.onAddSelectedFavorites}
              disabled={props.selectedPaths.length === 0}
            >
              {t('workspace.favorites.addSelection')}
            </button>
            <WorkspaceEmpty
              visible={props.favorites.length === 0}
              text={t('workspace.favorites.empty')}
            />
            <ul className="workspace-list">
              {props.favorites.map((favorite) => (
                <li key={favorite.path}>
                  <button
                    type="button"
                    className="workspace-list__primary"
                    aria-label={t('workspace.favorites.open')}
                    onClick={() => props.onOpenFavorite(favorite.path)}
                  >
                    <strong>{splitPath(favorite.path).name || favorite.path}</strong>
                    <span>{favorite.path}</span>
                  </button>
                  <button
                    type="button"
                    aria-label={t('workspace.favorites.reveal')}
                    onClick={() => props.onRevealFavorite(favorite.path)}
                  >
                    ↗
                  </button>
                  <button
                    type="button"
                    aria-label={t('workspace.remove')}
                    onClick={() => props.onRemoveFavorite(favorite.path)}
                  >
                    ×
                  </button>
                </li>
              ))}
            </ul>
          </>
        ) : null}

        {activeSection === 'coverage' ? (
          <div className="coverage-details">
            <CoverageRow
              label={t('workspace.coverage.permission')}
              value={
                coverage.fullDiskAccessGranted
                  ? t('workspace.coverage.granted')
                  : t('workspace.coverage.denied')
              }
            />
            <CoverageRow label={t('workspace.coverage.state')} value={coverage.lifecycleState} />
            <CoverageRow
              label={t('workspace.coverage.indexed')}
              value={coverage.scannedFiles.toLocaleString()}
            />
            <CoverageRow label={t('workspace.coverage.root')} value={coverage.watchRoot} />
            <PathGroup
              title={t('workspace.coverage.excluded')}
              paths={coverage.ignorePaths}
              empty={t('workspace.coverage.none')}
            />
            <PathGroup
              title={t('workspace.coverage.included')}
              paths={coverage.includePaths}
              empty={t('workspace.coverage.allUnderRoot')}
            />
            {coverage.currentDirectoryQuery ? (
              <CoverageRow
                label={t('workspace.coverage.directoryScope')}
                value={coverage.currentDirectoryQuery}
              />
            ) : null}
            <p className="workspace-note">{t('workspace.coverage.explanation')}</p>
            <div className="workspace-actions">
              <button type="button" onClick={props.onOpenPreferences}>
                {t('workspace.coverage.openSettings')}
              </button>
              <button type="button" onClick={operations.onRequestRescan}>
                {t('workspace.operations.rescan')}
              </button>
            </div>
          </div>
        ) : null}

        {activeSection === 'operations' ? (
          <div className="operation-list">
            <OperationRow
              label={t('workspace.operations.activity')}
              state={`${t(`workspace.operations.activityLevels.${operations.activity}`)} · ${t('workspace.operations.indexedCount', { count: operations.scannedFiles })}`}
            />
            <OperationRow
              label={t('workspace.operations.indexing')}
              state={
                operations.indexingPaused
                  ? t('workspace.operations.paused')
                  : operations.lifecycleState
              }
              actionLabel={
                operations.indexingPaused
                  ? t('workspace.operations.resumeIndexing')
                  : t('workspace.operations.pauseIndexing')
              }
              onAction={operations.onToggleIndexing}
            />
            {operations.isSearching ? (
              <OperationRow
                label={t('workspace.operations.search')}
                state={t('workspace.operations.running')}
                actionLabel={t('workspace.operations.cancelSearch')}
                onAction={operations.onCancelSearch}
              />
            ) : null}
            {operations.isSorting ? (
              <OperationRow
                label={t('workspace.operations.sorting')}
                state={t('workspace.operations.running')}
                actionLabel={t('workspace.operations.cancelSort')}
                onAction={operations.onCancelSort}
              />
            ) : null}
            {operations.fileOperations.map((operation) => (
              <OperationRow
                key={operation.id}
                label={t('workspace.operations.trash')}
                state={t(`workspace.operations.fileStates.${operation.state}`, {
                  count: operation.itemCount,
                })}
              />
            ))}
            {!operations.isSearching &&
            !operations.isSorting &&
            operations.lifecycleState === 'Ready' ? (
              <WorkspaceEmpty visible text={t('workspace.operations.idle')} />
            ) : null}
            <button
              type="button"
              className="workspace-primary-action"
              onClick={operations.onRequestRescan}
              disabled={operations.indexingPaused}
            >
              {t('workspace.operations.rescan')}
            </button>
          </div>
        ) : null}
      </div>
    </aside>
  );
}

function WorkspaceEmpty({ visible, text }: { visible: boolean; text: string }) {
  return visible ? <p className="workspace-empty">{text}</p> : null;
}

function CoverageRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="coverage-row">
      <span>{label}</span>
      <strong title={value}>{value}</strong>
    </div>
  );
}

function PathGroup({ title, paths, empty }: { title: string; paths: string[]; empty: string }) {
  return (
    <section className="coverage-paths">
      <h3>{title}</h3>
      {paths.length ? (
        <ul>
          {paths.map((path) => (
            <li key={path} title={path}>
              {path}
            </li>
          ))}
        </ul>
      ) : (
        <p>{empty}</p>
      )}
    </section>
  );
}

function OperationRow({
  label,
  state,
  actionLabel,
  onAction,
}: {
  label: string;
  state: string;
  actionLabel?: string;
  onAction?: () => void;
}) {
  return (
    <div className="operation-row">
      <div>
        <strong>{label}</strong>
        <span>{state}</span>
      </div>
      {actionLabel && onAction ? (
        <button type="button" onClick={onAction}>
          {actionLabel}
        </button>
      ) : null}
    </div>
  );
}
