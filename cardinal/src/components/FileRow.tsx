import React, { memo, useCallback, DragEvent, useRef } from 'react';
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
  onOpen?: (path: string) => void;
  onSelect: (
    rowIndex: number,
    options: { isShift: boolean; isMeta: boolean; isCtrl: boolean },
  ) => void;
  isSelected?: boolean;
  selectedPathsForDrag?: string[];
  caseInsensitive?: boolean;
  highlightTerms?: readonly string[];
};

export const FileRow = memo(function FileRow({
  item,
  rowIndex,
  style,
  onContextMenu,
  onOpen,
  onSelect,
  isSelected = false,
  selectedPathsForDrag = [],
  caseInsensitive,
  highlightTerms,
}: FileRowProps): React.JSX.Element | null {
  const pendingSelectRef = useRef<{
    isShift: boolean;
    isMeta: boolean;
    isCtrl: boolean;
  } | null>(null);

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
    if (onContextMenu) {
      onContextMenu(e, path ?? '', rowIndex);
    }
  };

  const handleMouseDown = (e: ReactMouseEvent<HTMLDivElement>) => {
    if (e.button !== 0) {
      return;
    }

    const options = {
      isShift: e.shiftKey,
      isMeta: e.metaKey,
      isCtrl: e.ctrlKey,
    };

    const hasModifier = options.isShift || options.isMeta || options.isCtrl;
    if (!isSelected || hasModifier) {
      onSelect(rowIndex, options);
      pendingSelectRef.current = null;
      return;
    }

    pendingSelectRef.current = options;
  };

  const handleMouseUp = (e: ReactMouseEvent<HTMLDivElement>) => {
    if (e.button !== 0) {
      return;
    }

    const pending = pendingSelectRef.current;
    if (!pending) {
      return;
    }

    pendingSelectRef.current = null;
    onSelect(rowIndex, pending);
  };

  const handleDoubleClick = (e: ReactMouseEvent<HTMLDivElement>) => {
    e.preventDefault();
    if (path && onOpen) {
      onOpen(path);
    }
  };

  const handleDragStart = useCallback(
    (e: DragEvent<HTMLDivElement>) => {
      if (!path) {
        return;
      }

      pendingSelectRef.current = null;

      const isDraggingSelected = isSelected && selectedPathsForDrag.length > 0;
      const pathsToDrag =
        isDraggingSelected && selectedPathsForDrag.length > 0 ? selectedPathsForDrag : [path];

      e.dataTransfer.effectAllowed = 'copy';
      e.dataTransfer.setData('text/plain', pathsToDrag.join('\n'));
      void startNativeFileDrag({ paths: pathsToDrag, icon: item.icon });
    },
    [isSelected, item.icon, path, selectedPathsForDrag],
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
      data-row-path={path ?? undefined}
      onContextMenu={handleContextMenu}
      onMouseDown={handleMouseDown}
      onMouseUp={handleMouseUp}
      onDoubleClick={handleDoubleClick}
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
