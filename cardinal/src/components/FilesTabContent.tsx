import React from 'react';
import type { CSSProperties, ReactNode } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import { ColumnHeader } from './ColumnHeader';
import { StateDisplay, type DisplayState } from './StateDisplay';
import { VirtualList } from './VirtualList';
import type { ColumnKey } from '../constants';
import type { VirtualListHandle } from './VirtualList';
import type { SearchResultItem } from '../types/search';
import type { SlabIndex } from '../types/slab';
import type { SortKey, SortState } from '../types/sort';
import type { ViewMode } from '../constants';
import { VirtualGrid, type VirtualGridHandle } from './VirtualGrid';
import { FileGridItem } from './FileGridItem';
import { startNativeFileDrag } from '../utils/drag';

type FilesTabContentProps = {
  headerRef: React.RefObject<HTMLDivElement>;
  onResizeStart: (columnKey: ColumnKey) => (event: ReactMouseEvent<HTMLSpanElement>) => void;
  onHeaderContextMenu?: (event: ReactMouseEvent<HTMLDivElement>) => void;
  displayState: DisplayState;
  searchErrorMessage: string | null;
  currentQuery: string;
  virtualListRef: React.RefObject<VirtualListHandle | VirtualGridHandle>;
  viewMode: ViewMode;
  iconSize: number;
  results: SlabIndex[];
  rowHeight: number;
  overscan: number;
  renderRow: (
    rowIndex: number,
    item: SearchResultItem | undefined,
    rowStyle: CSSProperties,
  ) => ReactNode;
  onScrollSync: (scrollLeft: number) => void;
  sortState?: SortState;
  onSortToggle?: (sortKey: SortKey) => void;
  sortDisabled?: boolean;
  sortIndicatorMode?: 'triangle' | 'circle';
  sortDisabledTooltip?: string | null;
  onSelect: (
    rowIndex: number,
    options: { isShift: boolean; isMeta: boolean; isCtrl: boolean },
  ) => void;
  onBulkSelect?: (indices: number[]) => void;
  onContextMenu: (event: ReactMouseEvent<HTMLDivElement>, path: string, rowIndex: number) => void;
  onOpen: (path: string) => void;
  onDragStart: (path: string, itemIsSelected: boolean, icon?: string) => void;
  caseInsensitive?: boolean;
  highlightTerms?: readonly string[];
  selectedIndices: number[];
  selectedPaths: string[];
};

export function FilesTabContent({
  headerRef,
  onResizeStart,
  onHeaderContextMenu,
  displayState,
  searchErrorMessage,
  currentQuery,
  virtualListRef,
  viewMode,
  iconSize,
  results,
  rowHeight,
  overscan,
  renderRow,
  onScrollSync,
  sortState,
  onSortToggle,
  sortDisabled = false,
  sortIndicatorMode = 'triangle',
  sortDisabledTooltip,
  onSelect,
  onBulkSelect,
  onContextMenu,
  onOpen,
  onDragStart,
  caseInsensitive,
  highlightTerms,
  selectedIndices,
  selectedPaths,
}: FilesTabContentProps): React.JSX.Element {
  const selectedIndexSet = React.useMemo(() => new Set(selectedIndices), [selectedIndices]);

  const renderGridItem = React.useCallback(
    (
      rowIndex: number,
      item: SearchResultItem | undefined,
      style: CSSProperties,
      actualItemWidth: number,
    ) => {
      if (!item) {
        return (
          <div
            key={`placeholder-${rowIndex}`}
            className="grid-item grid-item-loading"
            style={style}
          />
        );
      }

      return (
        <FileGridItem
          key={item.path}
          rowIndex={rowIndex}
          item={item}
          style={style}
          iconSize={iconSize}
          actualItemWidth={actualItemWidth}
          isSelected={selectedIndexSet.has(rowIndex)}
          onDragStart={onDragStart}
          caseInsensitive={caseInsensitive}
          highlightTerms={highlightTerms}
          onContextMenu={onContextMenu}
          onSelect={onSelect}
          onOpen={onOpen}
        />
      );
    },
    [
      caseInsensitive,
      highlightTerms,
      onContextMenu,
      onOpen,
      onSelect,
      selectedIndexSet,
      iconSize,
      onDragStart,
    ],
  );

  return (
    <div className="scroll-area">
      {viewMode === 'list' && (
        <ColumnHeader
          ref={headerRef}
          onResizeStart={onResizeStart}
          onContextMenu={onHeaderContextMenu}
          sortState={sortState}
          onSortToggle={onSortToggle}
          sortDisabled={sortDisabled}
          sortIndicatorMode={sortIndicatorMode}
          sortDisabledTooltip={sortDisabledTooltip}
        />
      )}
      <div className="flex-fill">
        {displayState !== 'results' ? (
          <StateDisplay state={displayState} message={searchErrorMessage} query={currentQuery} />
        ) : viewMode === 'list' ? (
          <VirtualList
            ref={virtualListRef as React.RefObject<VirtualListHandle>}
            results={results}
            rowHeight={rowHeight}
            overscan={overscan}
            renderRow={renderRow}
            onScrollSync={onScrollSync}
            className="virtual-list"
          />
        ) : (
          <VirtualGrid
            ref={virtualListRef as React.RefObject<VirtualGridHandle>}
            results={results}
            renderItem={renderGridItem}
            itemWidth={iconSize + 40}
            itemHeight={iconSize + 100}
            onBulkSelect={onBulkSelect}
            onSelect={onSelect}
            overscanRows={overscan}
            className="virtual-grid"
          />
        )}
      </div>
    </div>
  );
}
