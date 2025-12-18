import React, { memo, useCallback, DragEvent, useRef } from 'react';
import type { CSSProperties, MouseEvent as ReactMouseEvent } from 'react';
import { splitTextWithHighlights, applyMiddleEllipsis } from './MiddleEllipsisHighlight';
import type { SearchResultItem } from '../types/search';
import { startNativeFileDrag } from '../utils/drag';

type FileGridItemProps = {
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
  onDragStart?: (path: string, itemIsSelected: boolean, icon?: string) => void;
  caseInsensitive?: boolean;
  highlightTerms?: readonly string[];
  iconSize: number;
  actualItemWidth: number;
};

export const FileGridItem = memo(function FileGridItem({
  item,
  rowIndex,
  style,
  onContextMenu,
  onOpen,
  onSelect,
  isSelected = false,
  onDragStart,
  caseInsensitive,
  highlightTerms,
  iconSize,
  actualItemWidth,
}: FileGridItemProps): React.JSX.Element | null {
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

  if (path) {
    if (path === '/') {
      filename = '/';
    } else {
      const parts = path.split(/[\\/]/);
      filename = parts.pop() || '';
    }
  }

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
    (e: React.DragEvent<HTMLDivElement>) => {
      if (!path || !onDragStart) {
        return;
      }

      pendingSelectRef.current = null;

      const dataTransfer = e.dataTransfer;
      if (dataTransfer) {
        dataTransfer.effectAllowed = 'copy';
        // Note: System-level multi-drag will be handled by the native side via onDragStart.
        // We set dummy text data to satisfy web-standard DND if necessary.
        dataTransfer.setData('text/plain', path);
      }

      onDragStart(path, isSelected, item.icon);
    },
    [isSelected, item.icon, path, onDragStart],
  );

  const itemClassName = ['grid-item', isSelected ? 'grid-item-selected' : '']
    .filter(Boolean)
    .join(' ');

  const highlightedParts = React.useMemo(() => {
    if (!filename) return [];
    const parts = splitTextWithHighlights(filename, highlightTerms, { caseInsensitive });
    // Width of the text area is the actual column width minus local padding
    const textAreaWidth = actualItemWidth - 16;
    // 9.5px is an extreme buffer for 11px font width to strictly ensure 2 lines
    const charsPerLine = Math.floor(textAreaWidth / 9.5);
    const maxChars = charsPerLine * 2;
    return applyMiddleEllipsis(parts, maxChars);
  }, [filename, highlightTerms, caseInsensitive, actualItemWidth]);

  return (
    <div
      style={
        {
          ...style,
          '--grid-width': `${actualItemWidth}px`,
          '--icon-size': `${iconSize}px`,
        } as React.CSSProperties
      }
      className={itemClassName}
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
      <div className="grid-item-icon-container">
        {item.icon ? (
          <img src={item.icon} alt="icon" className="grid-item-icon" />
        ) : (
          <div className="grid-item-icon grid-item-icon-placeholder" aria-hidden="true" />
        )}
      </div>
      <div className="grid-item-name">
        <span className="grid-item-name-text">
          {highlightedParts.map((part, index) =>
            part.isHighlight ? (
              <strong key={`${part.text}-${index}`}>{part.text}</strong>
            ) : (
              <span key={`${part.text}-${index}`}>{part.text}</span>
            ),
          )}
        </span>
      </div>
    </div>
  );
});

FileGridItem.displayName = 'FileGridItem';
