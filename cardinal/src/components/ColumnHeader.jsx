import React, { forwardRef } from 'react';

const columns = [
  { key: 'filename', label: 'Filename', className: 'filename-text' },
  { key: 'path', label: 'Path', className: 'path-text' },
  { key: 'size', label: 'Size', className: 'size-text' },
  { key: 'modified', label: 'Modified', className: 'mtime-text' },
  { key: 'created', label: 'Created', className: 'ctime-text' }
];

// Column widths are applied via CSS vars on container; no need to pass colWidths prop.
export const ColumnHeader = forwardRef(({ onResizeStart, onContextMenu }, ref) => (
  <div ref={ref} className='header-row-container'>
    <div className='header-row columns' onContextMenu={onContextMenu}>
      {columns.map(({ key, label, className }) => (
        <span key={key} className={`${className} header header-cell`}>
          {label}
          <span className='col-resizer' onMouseDown={onResizeStart(key)} />
        </span>
      ))}
    </div>
  </div>
));
