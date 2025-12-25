import React, { memo } from 'react';
import type { CSSProperties, MouseEvent as ReactMouseEvent } from 'react';
import { FileRow } from './FileRow';
import type { SearchResultItem } from '../types/search';

type FileRowRendererProps = {
  rowIndex: number;
  item: SearchResultItem;
  style: CSSProperties;
  isSelected: boolean;
  onDragStart: (path: string, itemIsSelected: boolean, icon?: string) => void;
  caseInsensitive: boolean;
  highlightTerms: readonly string[];
  onContextMenu: (event: ReactMouseEvent<HTMLDivElement>, path: string, rowIndex: number) => void;
  onSelect: (
    rowIndex: number,
    options: { isShift: boolean; isMeta: boolean; isCtrl: boolean },
  ) => void;
  onOpen: (path: string) => void;
};

export const FileRowRenderer = memo(function FileRowRenderer({
  rowIndex,
  item,
  style,
  isSelected,
  onDragStart,
  caseInsensitive,
  highlightTerms,
  onContextMenu,
  onSelect,
  onOpen,
}: FileRowRendererProps) {
  return (
    <FileRow
      item={item}
      rowIndex={rowIndex}
      style={{ ...style, width: 'var(--columns-total)' }}
      onContextMenu={onContextMenu}
      onSelect={onSelect}
      onOpen={onOpen}
      isSelected={isSelected}
      onDragStart={onDragStart}
      caseInsensitive={caseInsensitive}
      highlightTerms={highlightTerms}
    />
  );
});
