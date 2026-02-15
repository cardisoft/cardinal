import React, { forwardRef } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import type { ColumnKey } from '../constants';
import type { SortKey, SortState } from '../types/sort';
import { useTranslation } from 'react-i18next';

const columns: Array<{ key: ColumnKey; labelKey: string; className: string }> = [
  { key: 'filename', labelKey: 'columns.filename', className: 'filename-text' },
  { key: 'path', labelKey: 'columns.path', className: 'path-text' },
  { key: 'size', labelKey: 'columns.size', className: 'size-text' },
  { key: 'modified', labelKey: 'columns.modified', className: 'mtime-text' },
  { key: 'created', labelKey: 'columns.created', className: 'ctime-text' },
];

const sortableColumns: Partial<Record<ColumnKey, SortKey>> = {
  filename: 'filename',
  path: 'fullPath',
  size: 'size',
  modified: 'mtime',
  created: 'ctime',
};

type ColumnHeaderProps = {
  onResizeStart: (columnKey: ColumnKey) => (event: ReactMouseEvent<HTMLSpanElement>) => void;
  onContextMenu: (event: ReactMouseEvent<HTMLDivElement>) => void;
  sortState: SortState;
  onSortToggle: (sortKey: SortKey) => void;
  sortDisabled: boolean;
  sortIndicatorMode: 'triangle' | 'circle';
  sortDisabledTooltip: string | null;
};

// Column widths are applied via CSS vars on container; no need to pass colWidths prop.
export const ColumnHeader = forwardRef<HTMLDivElement, ColumnHeaderProps>(
  (
    {
      onResizeStart,
      onContextMenu,
      sortState,
      onSortToggle,
      sortDisabled,
      sortIndicatorMode,
      sortDisabledTooltip,
    },
    ref,
  ) => {
    const { t } = useTranslation();
    return (
      <div ref={ref} className="header-row-container">
        <div className="header-row columns" onContextMenu={onContextMenu}>
          {columns.map(({ key, labelKey, className }) => {
            const label = t(labelKey);
            const sortKey = sortableColumns[key];
            const isActive = Boolean(sortKey && sortState?.key === sortKey);
            const indicatorClasses = ['sort-indicator'];

            if (sortIndicatorMode === 'circle') {
              indicatorClasses.push('sort-indicator--circle');
            } else if (isActive && sortState) {
              indicatorClasses.push(
                sortState.direction === 'asc' ? 'sort-indicator--asc' : 'sort-indicator--desc',
              );
            } else {
              indicatorClasses.push('sort-indicator--neutral');
            }

            if (sortDisabled) {
              indicatorClasses.push('sort-indicator--disabled');
            } else if (isActive) {
              indicatorClasses.push('sort-indicator--active');
            }

            const title = sortDisabled ? (sortDisabledTooltip ?? undefined) : undefined;

            if (!sortKey) {
              return (
                <span key={key} className={`${className} header header-cell`}>
                  {label}
                  <span
                    className="col-resizer"
                    onMouseDown={onResizeStart(key)} // consume column-specific resize closures from the parent hook
                  />
                </span>
              );
            }

            return (
              <span key={key} className={`${className} header header-cell`}>
                <button
                  type="button"
                  className="sort-button"
                  onClick={() => onSortToggle(sortKey)}
                  disabled={sortDisabled}
                  aria-pressed={isActive && !sortDisabled}
                  title={title}
                >
                  <span className="sort-button__label">{label}</span>
                  <span className={indicatorClasses.join(' ')} aria-hidden="true" />
                </button>
                <span
                  className="col-resizer"
                  onMouseDown={onResizeStart(key)} // consume column-specific resize closures from the parent hook
                />
              </span>
            );
          })}
          {/* Spacer for scrollbar width alignment */}
          <span className="header-scrollbar-spacer" />
        </div>
      </div>
    );
  },
);

ColumnHeader.displayName = 'ColumnHeader';
