import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { SearchFiltersController } from '../hooks/useSearchFilters';
import type { SearchFilter } from '../types/searchFilters';
import { decodeCsv, encodeCsv } from '../utils/csv';
import { parseFilterRows } from '../utils/filterCsv';
import { exportTextFile, importTextFile } from '../utils/fileTransfer';

type FiltersOverlayProps = {
  open: boolean;
  onClose: () => void;
  controller: SearchFiltersController;
};

type Draft = { name: string; search: string };

const emptyDraft = (): Draft => ({ name: '', search: '' });

/**
 * Filter preset manager: a list on the left and a vertical action column on the right
 * (new / edit / import / export / delete), mirroring the bookmark manager layout.
 */
export function FiltersOverlay({
  open,
  onClose,
  controller,
}: FiltersOverlayProps): React.JSX.Element | null {
  const { t } = useTranslation();
  const { filters, addFilter, updateFilter, deleteFilter, appendFilters } = controller;

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);

  useEffect(() => {
    if (!open) {
      return;
    }
    setDraft(null);
    setEditingId(null);
    setStatus(null);
    setSelectedId((prev) =>
      prev && filters.some((filter) => filter.id === prev) ? prev : (filters[0]?.id ?? null),
    );
    // Only resets on open; filters changes are handled below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  useEffect(() => {
    if (!selectedId) {
      return;
    }
    if (!filters.some((filter) => filter.id === selectedId)) {
      setSelectedId(filters[0]?.id ?? null);
    }
  }, [filters, selectedId]);

  useEffect(() => {
    if (!open) {
      return;
    }
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        if (draft) {
          setDraft(null);
          setEditingId(null);
          return;
        }
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [draft, onClose, open]);

  const selected = useMemo(
    () => filters.find((filter) => filter.id === selectedId) ?? null,
    [filters, selectedId],
  );

  const commitDraft = useCallback(() => {
    if (!draft) {
      return;
    }
    const name = draft.name.trim();
    if (!name) {
      return;
    }
    if (editingId) {
      updateFilter(editingId, { name, search: draft.search });
      setStatus(t('filters.saved'));
    } else {
      addFilter(name, draft.search);
      setStatus(t('filters.created'));
    }
    setDraft(null);
    setEditingId(null);
  }, [addFilter, draft, editingId, t, updateFilter]);

  const handleExport = useCallback(async () => {
    try {
      const rows: string[][] = [['name', 'search']];
      filters.forEach((filter) => rows.push([filter.name, filter.search]));
      const saved = await exportTextFile({
        defaultPath: 'cardinal-filters.csv',
        contents: encodeCsv(rows),
        filterName: t('filters.csvFileType'),
      });
      setStatus(saved ? t('filters.exported') : null);
    } catch (error) {
      console.error('Failed to export filters', error);
      setStatus(t('filters.ioFailed'));
    }
  }, [filters, t]);

  const handleImport = useCallback(async () => {
    try {
      const csv = await importTextFile(t('filters.csvFileType'));
      if (csv == null) {
        return;
      }
      const parsed = parseFilterRows(decodeCsv(csv));
      if (parsed.length === 0) {
        setStatus(t('filters.ioFailed'));
        return;
      }
      appendFilters(parsed);
      setStatus(t('filters.imported', { count: parsed.length }));
    } catch (error) {
      console.error('Failed to import filters', error);
      setStatus(t('filters.ioFailed'));
    }
  }, [appendFilters, t]);

  if (!open) {
    return null;
  }

  const handleOverlayClick = (event: React.MouseEvent<HTMLDivElement>) => {
    if (event.target === event.currentTarget) {
      onClose();
    }
  };

  const startCreate = () => {
    setDraft(emptyDraft());
    setEditingId(null);
  };

  const startEdit = (filter: SearchFilter) => {
    setSelectedId(filter.id);
    setDraft({ name: filter.name, search: filter.search });
    setEditingId(filter.id);
  };

  return (
    <div className="filters-overlay" role="dialog" aria-modal="true" onClick={handleOverlayClick}>
      <div className="filters-card">
        <header className="filters-card__header">
          <h1 className="filters-card__title">{t('filters.title')}</h1>
          <button
            type="button"
            className="bookmarks-close"
            onClick={onClose}
            aria-label={t('filters.close')}
          >
            ×
          </button>
        </header>

        <div className="filters-body">
          <div className="filters-table">
            <div className="filters-table__head">
              <span>{t('filters.columns.name')}</span>
              <span>{t('filters.columns.search')}</span>
            </div>
            <div className="filters-table__rows">
              {filters.map((filter) => (
                <div
                  key={filter.id}
                  className={`filters-row${filter.id === selectedId ? ' is-selected' : ''}`}
                  onClick={() => setSelectedId(filter.id)}
                  onDoubleClick={() => startEdit(filter)}
                  role="row"
                  tabIndex={0}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter') {
                      startEdit(filter);
                    }
                  }}
                >
                  <span className="filters-row__name">{filter.name}</span>
                  <span className="filters-row__search">{filter.search}</span>
                </div>
              ))}
            </div>
          </div>

          <div className="filters-actions">
            <button type="button" className="bookmarks-button" onClick={startCreate}>
              {t('filters.new')}
            </button>
            <button
              type="button"
              className="bookmarks-button"
              disabled={!selected}
              onClick={() => selected && startEdit(selected)}
            >
              {t('filters.edit')}
            </button>
            <button type="button" className="bookmarks-button" onClick={() => void handleImport()}>
              {t('filters.import')}
            </button>
            <button type="button" className="bookmarks-button" onClick={() => void handleExport()}>
              {t('filters.export')}
            </button>
            <button
              type="button"
              className="bookmarks-button bookmarks-button--danger"
              disabled={!selected}
              onClick={() => {
                if (selected) {
                  deleteFilter(selected.id);
                }
              }}
            >
              {t('filters.delete')}
            </button>
          </div>
        </div>

        {draft ? (
          <div className="filters-editor">
            <input
              className="bookmarks-input"
              value={draft.name}
              placeholder={t('filters.columns.name')}
              autoFocus
              onChange={(event) => setDraft((prev) => ({ ...prev!, name: event.target.value }))}
              onKeyDown={(event) => {
                if (event.key === 'Enter') {
                  commitDraft();
                }
              }}
            />
            <input
              className="bookmarks-input"
              value={draft.search}
              placeholder={t('filters.searchPlaceholder')}
              onChange={(event) => setDraft((prev) => ({ ...prev!, search: event.target.value }))}
              onKeyDown={(event) => {
                if (event.key === 'Enter') {
                  commitDraft();
                }
              }}
            />
            <button type="button" className="bookmarks-button" onClick={commitDraft}>
              {t('filters.save')}
            </button>
            <button
              type="button"
              className="bookmarks-button"
              onClick={() => {
                setDraft(null);
                setEditingId(null);
              }}
            >
              {t('filters.cancel')}
            </button>
          </div>
        ) : null}

        {status ? (
          <p className="bookmarks-status" role="status">
            {status}
          </p>
        ) : null}
      </div>
    </div>
  );
}

export default FiltersOverlay;
