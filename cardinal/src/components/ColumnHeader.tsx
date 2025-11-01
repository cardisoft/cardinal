import React, { forwardRef } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import type { ColumnKey } from '../constants';

const columns: Array<{ key: ColumnKey; label: string; className: string }> = [
  { key: 'filename', label: 'Filename', className: 'filename-text' },
  { key: 'path', label: 'Path', className: 'path-text' },
  { key: 'size', label: 'Size', className: 'size-text' },
  { key: 'modified', label: 'Modified', className: 'mtime-text' },
  { key: 'created', label: 'Created', className: 'ctime-text' },
];

type ColumnHeaderProps = {
  onResizeStart: (columnKey: ColumnKey) => (event: ReactMouseEvent<HTMLSpanElement>) => void;
  onContextMenu?: (event: ReactMouseEvent<HTMLDivElement>) => void;
};

// Column widths are applied via CSS vars on container; no need to pass colWidths prop.
export const ColumnHeader = forwardRef<HTMLDivElement, ColumnHeaderProps>(
  ({ onResizeStart, onContextMenu }, ref) => (
    <div ref={ref} className="header-row-container">
      <div className="header-row columns" onContextMenu={onContextMenu}>
        {columns.map(({ key, label, className }) => (
          <span key={key} className={`${className} header header-cell`}>
            {label}
            <span
              className="col-resizer"
              onMouseDown={onResizeStart(key)} // consume column-specific resize closures from the parent hook
            />
          </span>
        ))}
        {/* Spacer for scrollbar width alignment */}
        <span className="header-scrollbar-spacer" />
      </div>
    </div>
  ),
);

ColumnHeader.displayName = 'ColumnHeader';
