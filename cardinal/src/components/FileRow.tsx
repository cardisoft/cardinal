import React, { memo, useCallback, DragEvent } from 'react';
import type { CSSProperties, MouseEvent as ReactMouseEvent } from 'react';
import { MiddleEllipsisHighlight } from './MiddleEllipsisHighlight';
import { formatKB, formatTimestamp } from '../utils/format';
import type { SearchResultItem } from '../types/search';
import { startNativeFileDrag } from '../utils/drag';

type FileRowProps = {
  item?: SearchResultItem;
  rowIndex: number;
  style?: CSSProperties;
  onContextMenu?: (event: ReactMouseEvent<HTMLDivElement>, path: string, rowIndex: number) => void;
  onSelect?: (path: string, rowIndex: number) => void;
  isSelected?: boolean;
  caseInsensitive?: boolean;
  highlightTerms?: readonly string[];
};

export const FileRow = memo(function FileRow({
  item,
  rowIndex,
  style,
  onContextMenu,
  onSelect,
  isSelected = false,
  caseInsensitive,
  highlightTerms,
}: FileRowProps): React.JSX.Element | null {
  if (!item) {
    return null;
  }

  const path = item.path;
  let filename = '';
  let directoryPath = '';

  if (path) {
    if (path === '/') {
      directoryPath = '/';
    } else {
      // Split on either slash to support Windows and POSIX paths.
      const parts = path.split(/[\\/]/);
      filename = parts.pop() || '';
      directoryPath = parts.join('/');
    }
  }

  const metadata = item.metadata;
  const mtimeSec = metadata?.mtime ?? item.mtime;
  const ctimeSec = metadata?.ctime ?? item.ctime;
  const sizeBytes = metadata?.size ?? item.size;
  const sizeText = metadata?.type !== 1 ? formatKB(sizeBytes) : null;
  const mtimeText = formatTimestamp(mtimeSec);
  const ctimeText = formatTimestamp(ctimeSec);

  const handleContextMenu = (e: ReactMouseEvent<HTMLDivElement>) => {
    e.preventDefault();
    if (path && onContextMenu) {
      onContextMenu(e, path, rowIndex);
    }
  };

  const handleMouseDown = (e: ReactMouseEvent<HTMLDivElement>) => {
    // Select on mouse down for responsiveness.
    if (path && onSelect && e.button === 0) {
      onSelect(path, rowIndex);
    }
  };

  const handleClick = () => {
    // onSelect is also here to support accessibility and other click triggers.
    if (path && onSelect) {
      onSelect(path, rowIndex);
    }
  };

  const handleDragStart = useCallback(
    (e: DragEvent<HTMLDivElement>) => {
      if (!path) {
        return;
      }
      e.dataTransfer.effectAllowed = 'copy';
      e.dataTransfer.setData('text/plain', path);
      void startNativeFileDrag({ paths: [path], icon: item.icon });
    },
    [item.icon, path],
  );

  const rowClassName = [
    'row',
    'columns',
    rowIndex % 2 === 0 ? 'row-even' : 'row-odd',
    isSelected ? 'row-selected' : '',
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <div
      style={style}
      className={rowClassName}
      onContextMenu={handleContextMenu}
      onMouseDown={handleMouseDown}
      onClick={handleClick}
      draggable={true}
      onDragStart={handleDragStart}
      aria-selected={isSelected}
      title={path}
    >
      <div className="filename-column">
        {item.icon ? (
          <img src={item.icon} alt="icon" className="file-icon" />
        ) : (
          <span className="file-icon file-icon-placeholder" aria-hidden="true" />
        )}
        <MiddleEllipsisHighlight
          className="filename-text"
          text={filename}
          highlightTerms={highlightTerms}
          caseInsensitive={caseInsensitive}
        />
      </div>
      {/* Directory column renders the parent path (the filename column already shows the leaf). */}
      <span className="path-text" title={directoryPath}>
        {directoryPath}
      </span>
      <span className={`size-text ${!sizeText ? 'muted' : ''}`}>{sizeText || '—'}</span>
      <span className={`mtime-text ${!mtimeText ? 'muted' : ''}`}>{mtimeText || '—'}</span>
      <span className={`ctime-text ${!ctimeText ? 'muted' : ''}`}>{ctimeText || '—'}</span>
    </div>
  );
});

FileRow.displayName = 'FileRow';
