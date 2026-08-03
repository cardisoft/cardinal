import React, { forwardRef, useCallback, useState } from 'react';
import type { DragEvent } from 'react';
import { useTranslation } from 'react-i18next';
import type { ColumnKey } from '../constants';
import type { SortKey, SortState } from '../types/sort';
import { CONTEXT_COLUMN, type OrderedColumn } from '../hooks/useColumnOrder';

const columnMeta: Record<ColumnKey, { labelKey: string; className: string }> = {
  filename: { labelKey: 'columns.filename', className: 'filename-text' },
  path: { labelKey: 'columns.path', className: 'path-text' },
  size: { labelKey: 'columns.size', className: 'size-text' },
  modified: { labelKey: 'columns.modified', className: 'mtime-text' },
  created: { labelKey: 'columns.created', className: 'ctime-text' },
};

const sortableColumns: Record<ColumnKey, SortKey> = {
  filename: 'filename',
  path: 'fullPath',
  size: 'size',
  modified: 'mtime',
  created: 'ctime',
};

type ColumnHeaderProps = {
  onResizeStart: (key: ColumnKey) => (event: React.MouseEvent<HTMLSpanElement>) => void;
  onContextMenu: (event: React.MouseEvent<HTMLDivElement>) => void;
  sortState: SortState | null;
  onSortToggle: (sortKey: SortKey) => void;
  sortDisabled: boolean;
  sortDisabledTooltip: string | null;
  showContentContext: boolean;
  columnOrder: readonly OrderedColumn[];
  onColumnMove: (dragged: OrderedColumn, target: OrderedColumn) => void;
  columnsTemplate: string;
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
      sortDisabledTooltip,
      showContentContext,
      columnOrder,
      onColumnMove,
      columnsTemplate,
    },
    ref,
  ) => {
    const { t } = useTranslation();
    const [draggedColumn, setDraggedColumn] = useState<OrderedColumn | null>(null);
    const [dropTarget, setDropTarget] = useState<OrderedColumn | null>(null);

    const handleDragStart = useCallback(
      (column: OrderedColumn) => (event: DragEvent<HTMLSpanElement>) => {
        setDraggedColumn(column);
        // Firefox ignores a drag that carries no data, and "move" gets the right cursor.
        event.dataTransfer.setData('text/plain', column);
        event.dataTransfer.effectAllowed = 'move';
      },
      [],
    );

    const handleDragOver = useCallback(
      (column: OrderedColumn) => (event: DragEvent<HTMLSpanElement>) => {
        if (!draggedColumn) {
          return;
        }
        // Without preventDefault the browser refuses the drop outright.
        event.preventDefault();
        event.dataTransfer.dropEffect = 'move';
        setDropTarget(column);
      },
      [draggedColumn],
    );

    const handleDrop = useCallback(
      (column: OrderedColumn) => (event: DragEvent<HTMLSpanElement>) => {
        event.preventDefault();
        if (draggedColumn) {
          onColumnMove(draggedColumn, column);
        }
        setDraggedColumn(null);
        setDropTarget(null);
      },
      [draggedColumn, onColumnMove],
    );

    const handleDragEnd = useCallback(() => {
      setDraggedColumn(null);
      setDropTarget(null);
    }, []);

    const dragClasses = (column: OrderedColumn) =>
      [
        draggedColumn === column ? 'header-cell--dragging' : '',
        dropTarget === column && draggedColumn !== column ? 'header-cell--drop-target' : '',
      ]
        .filter(Boolean)
        .join(' ');

    return (
      <div ref={ref} className="header-row-container">
        <div
          className="header-row columns"
          style={{ gridTemplateColumns: columnsTemplate }}
          onContextMenu={onContextMenu}
        >
          {columnOrder.map((column) => {
            if (column === CONTEXT_COLUMN) {
              if (!showContentContext) {
                return null;
              }
              // No sort key and no resizer: the snippet column's width follows the window.
              return (
                <span
                  key={column}
                  className={`context-text header header-cell ${dragClasses(column)}`}
                  draggable
                  onDragStart={handleDragStart(column)}
                  onDragOver={handleDragOver(column)}
                  onDrop={handleDrop(column)}
                  onDragEnd={handleDragEnd}
                >
                  {t('columns.context')}
                </span>
              );
            }

            const { labelKey, className } = columnMeta[column];
            const label = t(labelKey);
            const sortKey = sortableColumns[column];
            const isActive = sortState?.key === sortKey;
            const indicatorClasses = ['sort-indicator'];

            if (isActive && sortState) {
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

            const title = sortDisabled ? sortDisabledTooltip || undefined : undefined;

            return (
              <span
                key={column}
                className={`${className} header header-cell ${dragClasses(column)}`}
                draggable
                onDragStart={handleDragStart(column)}
                onDragOver={handleDragOver(column)}
                onDrop={handleDrop(column)}
                onDragEnd={handleDragEnd}
              >
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
                  // Dragging the resizer must not start a column drag: the handle sits inside the
                  // draggable header cell, and the browser hands the gesture to the parent.
                  draggable={false}
                  onDragStart={(event) => event.preventDefault()}
                  onMouseDown={onResizeStart(column)} // consume column-specific resize closures from the parent hook
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
