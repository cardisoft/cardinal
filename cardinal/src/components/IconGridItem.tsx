import React, { memo, useCallback, useRef, useState } from 'react';
import type { CSSProperties, MouseEvent as ReactMouseEvent } from 'react';
import type { SearchResultItem } from '../types/search';
import { splitPath } from '../utils/path';
import { startNativeFileDrag } from '../utils/drag';
import { hasModifierKey } from '../utils/keyboard';

type IconGridItemProps = {
  item: SearchResultItem;
  rowIndex: number;
  style?: CSSProperties;
  isSelected?: boolean;
  selectedPathsForDrag?: string[];
  onSelect: (
    rowIndex: number,
    options: { isShift: boolean; isMeta: boolean; isCtrl: boolean },
  ) => void;
  onOpen?: (path: string) => void;
  onContextMenu?: (event: ReactMouseEvent<HTMLDivElement>, path: string, rowIndex: number) => void;
};

/**
 * Height/width ratio at or above which artwork counts as "tall".
 *
 * NSWorkspace rasterises document/folder icons on a square canvas (ratio ~1.0), which leaves no
 * vertical slack in the icon box; a QuickLook video cover is 16:9 (ratio ~0.56) and has plenty.
 * Tall artwork therefore gets two text lines and wide artwork keeps three, so that a tile never
 * has to choose between squeezing its icon and leaving a gap.
 */
const TALL_ARTWORK_RATIO = 0.9;

export const IconGridItem = memo(function IconGridItem({
  item,
  rowIndex,
  style,
  isSelected = false,
  selectedPathsForDrag = [],
  onSelect,
  onOpen,
  onContextMenu,
}: IconGridItemProps): React.JSX.Element {
  const pendingSelectRef = useRef<{
    isShift: boolean;
    isMeta: boolean;
    isCtrl: boolean;
  } | null>(null);

  const path = item.path;
  const filename = path === '/' ? '' : splitPath(path).name;

  // Square document/folder icons fill the icon box vertically, so a third text line would
  // squeeze the icon and break the row's alignment. Those tiles get two lines; wide artwork
  // (16:9 covers) has the slack for three.
  const [tallArtwork, setTallArtwork] = useState(false);
  const handleIconLoad = useCallback((event: React.SyntheticEvent<HTMLImageElement>) => {
    const { naturalWidth, naturalHeight } = event.currentTarget;
    if (naturalWidth > 0 && naturalHeight > 0) {
      setTallArtwork(naturalHeight / naturalWidth >= TALL_ARTWORK_RATIO);
    }
  }, []);

  const handleMouseDown = (event: ReactMouseEvent<HTMLDivElement>) => {
    if (event.button !== 0) {
      return;
    }
    const options = {
      isShift: event.shiftKey,
      isMeta: event.metaKey,
      isCtrl: event.ctrlKey,
    };
    if (!isSelected || hasModifierKey(event, { includeAlt: false })) {
      onSelect(rowIndex, options);
      pendingSelectRef.current = null;
      return;
    }
    pendingSelectRef.current = options;
  };

  const handleMouseUp = (event: ReactMouseEvent<HTMLDivElement>) => {
    if (event.button !== 0) {
      return;
    }
    const pending = pendingSelectRef.current;
    if (!pending) {
      return;
    }
    pendingSelectRef.current = null;
    onSelect(rowIndex, pending);
  };

  const handleDoubleClick = (event: ReactMouseEvent<HTMLDivElement>) => {
    event.preventDefault();
    if (path && onOpen) {
      onOpen(path);
    }
  };

  const handleDragStart = useCallback(
    (event: React.DragEvent<HTMLDivElement>) => {
      if (!path) {
        return;
      }
      pendingSelectRef.current = null;
      const dataTransfer = event.dataTransfer;
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

  const handleContextMenu = (event: ReactMouseEvent<HTMLDivElement>) => {
    event.preventDefault();
    onContextMenu?.(event, path, rowIndex);
  };

  const className = ['icon-grid-item', isSelected ? 'is-selected' : ''].filter(Boolean).join(' ');

  return (
    <div
      style={style}
      className={className}
      data-row-path={path}
      onMouseDown={handleMouseDown}
      onMouseUp={handleMouseUp}
      onDoubleClick={handleDoubleClick}
      onContextMenu={handleContextMenu}
      draggable={true}
      onDragStart={handleDragStart}
      aria-selected={isSelected}
      title={path}
    >
      <div className="icon-grid-item__icon">
        {item.icon ? (
          <img
            src={item.icon}
            alt=""
            draggable={false}
            onLoad={handleIconLoad}
            onError={() => setTallArtwork(false)}
          />
        ) : (
          <span className="file-icon file-icon-placeholder" aria-hidden="true" />
        )}
      </div>
      {/* The wrapper reserves the label area and bottom-aligns the text, so tiles with one-line
          and three-line names share the same text baseline. */}
      <span className="icon-grid-item__label" data-lines={tallArtwork ? 'two' : 'three'}>
        <span className="icon-grid-item__label-text">{filename}</span>
      </span>
    </div>
  );
});

IconGridItem.displayName = 'IconGridItem';

export default IconGridItem;
