import React, { useCallback, useRef, memo, useEffect, useImperativeHandle, forwardRef } from 'react';
import AutoSizer from 'react-virtualized/dist/commonjs/AutoSizer';
import List from 'react-virtualized/dist/commonjs/List';
import 'react-virtualized/styles.css';
import { ROW_HEIGHT } from '../constants';
import { MiddleEllipsisHighlight } from './MiddleEllipsisHighlight';
import { formatTimestamp } from '../utils/format';

const COLUMNS = [
  { key: 'time', label: 'Time' },
  { key: 'name', label: 'Filename' },
  { key: 'path', label: 'Path' },
];

const BOTTOM_THRESHOLD = 50; // pixels from bottom to consider "at bottom"

const splitPath = (path) => {
  if (!path) {
    return { name: '—', directory: '' };
  }
  const normalized = path.replace(/\\/g, '/');
  if (normalized === '/') {
    return { name: '/', directory: '/' };
  }
  const slashIndex = normalized.lastIndexOf('/');
  if (slashIndex === -1) {
    return { name: normalized, directory: '' };
  }
  const directory = normalized.slice(0, slashIndex) || '/';
  const name = normalized.slice(slashIndex + 1) || normalized;
  return { name, directory };
};

// EventRow component for rendering individual rows
const EventRow = memo(function EventRow({ item: event, rowIndex, style, onContextMenu, searchQuery, caseInsensitive }) {
  const pathSource = event?.path ?? '';
  const { name, directory } = splitPath(pathSource);
  const timestamp = event?.timestamp;
  
  const formattedDate = formatTimestamp(timestamp) || '—';

  const handleContextMenu = useCallback(
    (e) => {
      if (pathSource && onContextMenu) {
        onContextMenu(e, pathSource);
      }
    },
    [pathSource, onContextMenu],
  );

  return (
    <div
      style={style}
      className={`row columns-events ${rowIndex % 2 === 0 ? 'row-even' : 'row-odd'}`}
      title={pathSource}
      onContextMenu={handleContextMenu}
    >
      <div className="event-time-column">
        <span className="event-time-primary">{formattedDate}</span>
      </div>
      <div className="event-name-column">
        <MiddleEllipsisHighlight
          text={name || '—'}
          className="event-name-text"
          highlightTerm={searchQuery}
          caseInsensitive={caseInsensitive}
        />
      </div>
      <span className="event-path-text" title={directory}>
        {directory || (pathSource ? '/' : '—')}
      </span>
    </div>
  );
});

const FSEventsPanel = forwardRef(({
  events,
  onResizeStart,
  onContextMenu,
  onHeaderContextMenu,
  searchQuery,
  caseInsensitive,
}, ref) => {
  const headerRef = useRef(null);
  const listRef = useRef(null);
  const isAtBottomRef = useRef(true); // Track if user is viewing bottom
  const prevEventsLengthRef = useRef(events.length);

  // Expose scrollToBottom method to parent
  useImperativeHandle(ref, () => ({
    scrollToBottom: () => {
      const list = listRef.current;
      if (!list || events.length === 0) return;
      
      list.scrollToRow(events.length - 1);
      isAtBottomRef.current = true; // Mark as at bottom
    }
  }), [events.length]);

  // Handle scrolling - sync horizontal scroll to header and track bottom position
  const handleScroll = useCallback(({ scrollLeft, scrollTop, scrollHeight, clientHeight }) => {
    // Check if user is at or near the bottom
    const distanceFromBottom = scrollHeight - (scrollTop + clientHeight);
    isAtBottomRef.current = distanceFromBottom <= BOTTOM_THRESHOLD;
  }, []);

  // Set up scroll listener on the actual Grid element for horizontal scroll sync
  useEffect(() => {
    const list = listRef.current;
    if (!list || !list.Grid) return;

    const gridElement = list.Grid._scrollingContainer;
    if (!gridElement) return;

    const handleHorizontalScroll = () => {
      if (headerRef.current && gridElement) {
        headerRef.current.scrollLeft = gridElement.scrollLeft;
      }
    };

    gridElement.addEventListener('scroll', handleHorizontalScroll);
    return () => {
      gridElement.removeEventListener('scroll', handleHorizontalScroll);
    };
  }, []);

  // Render individual row
  const rowRenderer = useCallback(
    ({ index, key, style }) => {
      const event = events[index];
      return (
        <EventRow
          key={key}
          item={event}
          rowIndex={index}
          style={{ ...style, width: 'var(--columns-events-total)' }}
          onContextMenu={onContextMenu}
          searchQuery={searchQuery}
          caseInsensitive={caseInsensitive}
        />
      );
    },
    [events, onContextMenu, searchQuery, caseInsensitive],
  );

  // Auto-scroll to bottom when new events arrive if user is at bottom
  useEffect(() => {
    const prevLength = prevEventsLengthRef.current;
    const currentLength = events.length;

    // Update the ref for next time
    prevEventsLengthRef.current = currentLength;

    // Only auto-scroll if:
    // 1. There are new events (length increased)
    // 2. User was at the bottom
    if (currentLength > prevLength && isAtBottomRef.current) {
      const list = listRef.current;
      if (list && currentLength > 0) {
        // Use queueMicrotask to ensure List has updated
        queueMicrotask(() => {
          if (listRef.current) {
            listRef.current.scrollToRow(currentLength - 1);
          }
        });
      }
    }
  }, [events.length]);

  return (
    <div className="events-panel-wrapper">
      <div ref={headerRef} className="header-row-container">
        <div className="header-row columns-events" onContextMenu={onHeaderContextMenu}>
          {COLUMNS.map(({ key, label }, index) => (
            <span key={key} className={`event-${key}-header header header-cell`}>
              {label}
              <span
                className="col-resizer"
                onMouseDown={(e) => onResizeStart(e, key)}
                role="separator"
                aria-orientation="vertical"
              />
            </span>
          ))}
        </div>
      </div>
      <div className="flex-fill">
        {events.length === 0 ? (
          <div className="events-empty" role="status">
            <p>No recent file events yet.</p>
            <p className="events-empty__hint">Keep working and check back for updates.</p>
          </div>
        ) : (
          <AutoSizer>
            {({ width, height }) => (
              <List
                ref={listRef}
                width={width}
                height={height}
                rowCount={events.length}
                rowHeight={ROW_HEIGHT}
                rowRenderer={rowRenderer}
                onScroll={handleScroll}
                overscanRowCount={10}
                className="events-list"
              />
            )}
          </AutoSizer>
        )}
      </div>
    </div>
  );
});

FSEventsPanel.displayName = 'FSEventsPanel';

export default FSEventsPanel;
