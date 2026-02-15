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

type FilesTabContentProps = {
  headerRef: React.Ref<HTMLDivElement>;
  onResizeStart: (columnKey: ColumnKey) => (event: ReactMouseEvent<HTMLSpanElement>) => void;
  onHeaderContextMenu: (event: ReactMouseEvent<HTMLDivElement>) => void;
  displayState: DisplayState;
  searchErrorMessage: string | null;
  currentQuery: string;
  virtualListRef: React.Ref<VirtualListHandle>;
  results: SlabIndex[];
  dataResultsVersion: number;
  displayedResultsVersion: number;
  rowHeight: number;
  overscan: number;
  renderRow: (
    rowIndex: number,
    item: SearchResultItem | undefined,
    rowStyle: CSSProperties,
  ) => ReactNode;
  onScrollSync: (scrollLeft: number) => void;
  sortState: SortState;
  onSortToggle: (sortKey: SortKey) => void;
  sortDisabled: boolean;
  sortDisabledTooltip: string | null;
};

export function FilesTabContent({
  headerRef,
  onResizeStart,
  onHeaderContextMenu,
  displayState,
  searchErrorMessage,
  currentQuery,
  virtualListRef,
  results,
  dataResultsVersion,
  displayedResultsVersion,
  rowHeight,
  overscan,
  renderRow,
  onScrollSync,
  sortState,
  onSortToggle,
  sortDisabled,
  sortDisabledTooltip,
}: FilesTabContentProps): React.JSX.Element {
  return (
    <div className="scroll-area">
      <ColumnHeader
        ref={headerRef}
        onResizeStart={onResizeStart}
        onContextMenu={onHeaderContextMenu}
        sortState={sortState}
        onSortToggle={onSortToggle}
        sortDisabled={sortDisabled}
        sortDisabledTooltip={sortDisabledTooltip}
      />
      <div className="flex-fill">
        {displayState !== 'results' ? (
          <StateDisplay state={displayState} message={searchErrorMessage} query={currentQuery} />
        ) : (
          <VirtualList
            ref={virtualListRef}
            results={results}
            dataResultsVersion={dataResultsVersion}
            displayedResultsVersion={displayedResultsVersion}
            rowHeight={rowHeight}
            overscan={overscan}
            renderRow={renderRow}
            onScrollSync={onScrollSync}
          />
        )}
      </div>
    </div>
  );
}
