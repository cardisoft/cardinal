import React from 'react';
import type { CSSProperties, ReactNode } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import { ColumnHeader } from './ColumnHeader';
import { StateDisplay, type DisplayState } from './StateDisplay';
import { VirtualList } from './VirtualList';
import { IconGrid } from './IconGrid';
import type { ColumnKey } from '../constants';
import type { VirtualListHandle } from './VirtualList';
import type { IconGridHandle, IconSizePreset } from './IconGrid';
import type { ResultsViewHandle, ResultsViewMode } from '../types/viewResults';
import type { SearchResultItem } from '../types/search';
import type { SlabIndex } from '../types/slab';
import type { SortKey, SortState } from '../types/sort';

type FilesTabContentProps = {
  headerRef: React.Ref<HTMLDivElement>;
  onResizeStart: (columnKey: ColumnKey) => (event: ReactMouseEvent<HTMLSpanElement>) => void;
  onHeaderContextMenu: (event: ReactMouseEvent<HTMLDivElement>) => void;
  displayState: DisplayState;
  /** Message shown by the loading/indexing/error panels. */
  stateMessage: string | null;
  /** Title override for the indexing panel (names the folder being indexed). */
  stateTitle: string | null;
  /** Folder scope the backend reported as unavailable. */
  unavailableScope: string | null;
  currentQuery: string;
  currentDirectoryQuery: string;
  resultsViewRef: React.Ref<ResultsViewHandle>;
  results: SlabIndex[];
  // Bumps when backend search results change; useDataLoader treats it as a data-cache reset.
  dataResultsVersion: number;
  // Bumps when visible ordering changes; VirtualList uses it for viewport-dependent refresh work.
  displayedResultsVersion: number;
  rowHeight: number;
  overscan: number;
  renderRow: (
    rowIndex: number,
    item: SearchResultItem | undefined,
    rowStyle: CSSProperties,
  ) => ReactNode;
  renderGridItem: (
    rowIndex: number,
    item: SearchResultItem | undefined,
    itemStyle: CSSProperties,
  ) => ReactNode;
  onScrollSync: (scrollLeft: number) => void;
  sortState: SortState;
  onSortToggle: (sortKey: SortKey) => void;
  sortDisabled: boolean;
  sortDisabledTooltip: string | null;
  viewMode: ResultsViewMode;
  iconSize: IconSizePreset;
  /** Base UI font size in px; the tiled view reserves 3 label lines from it. */
  fontSize: number;
  /** Requests the next page when the view nears the bottom (launch list paging). */
  onNearEnd?: () => void;
};

export function FilesTabContent({
  headerRef,
  onResizeStart,
  onHeaderContextMenu,
  displayState,
  stateMessage,
  stateTitle,
  unavailableScope,
  currentQuery,
  currentDirectoryQuery,
  resultsViewRef,
  results,
  dataResultsVersion,
  displayedResultsVersion,
  rowHeight,
  overscan,
  renderRow,
  renderGridItem,
  onScrollSync,
  sortState,
  onSortToggle,
  sortDisabled,
  sortDisabledTooltip,
  viewMode,
  iconSize,
  fontSize,
  onNearEnd,
}: FilesTabContentProps): React.JSX.Element {
  return (
    <div className="scroll-area">
      {viewMode === 'list' ? (
        <ColumnHeader
          ref={headerRef}
          onResizeStart={onResizeStart}
          onContextMenu={onHeaderContextMenu}
          sortState={sortState}
          onSortToggle={onSortToggle}
          sortDisabled={sortDisabled}
          sortDisabledTooltip={sortDisabledTooltip}
        />
      ) : null}
      <div className="flex-fill">
        {displayState !== 'results' ? (
          <StateDisplay
            state={displayState}
            message={stateMessage}
            title={stateTitle}
            scope={unavailableScope}
            query={currentQuery}
            directoryQuery={currentDirectoryQuery}
          />
        ) : viewMode === 'grid' ? (
          <IconGrid
            ref={resultsViewRef as React.Ref<IconGridHandle>}
            results={results}
            dataResultsVersion={dataResultsVersion}
            displayedResultsVersion={displayedResultsVersion}
            iconSize={iconSize}
            fontSize={fontSize}
            renderItem={renderGridItem}
            onNearEnd={onNearEnd}
          />
        ) : (
          <VirtualList
            ref={resultsViewRef as React.Ref<VirtualListHandle>}
            results={results}
            dataResultsVersion={dataResultsVersion}
            displayedResultsVersion={displayedResultsVersion}
            rowHeight={rowHeight}
            overscan={overscan}
            renderRow={renderRow}
            onScrollSync={onScrollSync}
            onNearEnd={onNearEnd}
          />
        )}
      </div>
    </div>
  );
}
