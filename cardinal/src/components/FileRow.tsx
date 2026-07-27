import React, { memo, useCallback, DragEvent, useRef } from 'react';
import type { CSSProperties, MouseEvent as ReactMouseEvent } from 'react';
import { MiddleEllipsisHighlight } from './MiddleEllipsisHighlight';
import { formatKB, formatTimestamp } from '../utils/format';
import type { SearchResultItem } from '../types/search';
import { startNativeFileDrag } from '../utils/drag';
import { splitPath } from '../utils/path';
import { hasModifierKey } from '../utils/keyboard';

type FileRowProps = {
  item: SearchResultItem;
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
  treeDepth?: number;
  isExpandable?: boolean;
  isExpanded?: boolean;
  onToggleExpanded?: () => void;
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
  treeDepth = 0,
  isExpandable = false,
  isExpanded = false,
  onToggleExpanded,
}: FileRowProps): React.JSX.Element {
  const pendingSelectRef = useRef<{
    isShift: boolean;
    isMeta: boolean;
    isCtrl: boolean;
  } | null>(null);

  const path = item.path;
  const pathParts = splitPath(path);
  const filename = path === '/' ? '' : pathParts.name;
  const directoryPath = pathParts.directory;

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
      onContextMenu(e, path, rowIndex);
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

    if (!isSelected || hasModifierKey(e, { includeAlt: false })) {
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

  const handleToggleExpanded = (e: ReactMouseEvent<HTMLButtonElement>) => {
    e.preventDefault();
    e.stopPropagation();
    onToggleExpanded?.();
  };

  const handleDragStart = useCallback(
    (e: DragEvent<HTMLDivElement>) => {
      if (!path) {
        return;
      }

      pendingSelectRef.current = null;

      const dataTransfer = e.dataTransfer;
      if (!dataTransfer) {
        return;
      }

      const isDraggingSelected = isSelected && selectedPathsForDrag.length > 0;
      const pathsToDrag = isDraggingSelected ? selectedPathsForDrag : [path];

      dataTransfer.effectAllowed = 'copy';
      dataTransfer.setData('text/plain', pathsToDrag.join('\n'));
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
      data-row-path={path}
      onContextMenu={handleContextMenu}
      onMouseDown={handleMouseDown}
      onMouseUp={handleMouseUp}
      onDoubleClick={handleDoubleClick}
      draggable={true}
      onDragStart={handleDragStart}
      aria-selected={isSelected}
      title={path}
    >
      <div
        className="filename-column"
        style={{ paddingLeft: `calc(var(--cell-hpad) + ${treeDepth * 16}px)` }}
      >
        {isExpandable ? (
          <button
            type="button"
            className={`disclosure-button${isExpanded ? ' disclosure-button--expanded' : ''}`}
            aria-label={isExpanded ? '折叠文件夹' : '展开文件夹'}
            aria-expanded={isExpanded}
            onMouseDown={(e) => e.stopPropagation()}
            onMouseUp={(e) => e.stopPropagation()}
            onDoubleClick={(e) => e.stopPropagation()}
            onClick={handleToggleExpanded}
          >
            <span aria-hidden="true">▶</span>
          </button>
        ) : (
          <span className="disclosure-spacer" aria-hidden="true" />
        )}
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
