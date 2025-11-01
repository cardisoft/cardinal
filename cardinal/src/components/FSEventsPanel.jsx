import React, { useCallback, useRef, memo, useEffect } from 'react';
import AutoSizer from 'react-virtualized/dist/commonjs/AutoSizer';
import List from 'react-virtualized/dist/commonjs/List';
import 'react-virtualized/styles.css';
import { ROW_HEIGHT } from '../constants';
import { MiddleEllipsisHighlight } from './MiddleEllipsisHighlight';

const COLUMNS = [
  { key: 'name', label: 'Filename' },
  { key: 'path', label: 'Path' },
  { key: 'time', label: 'Time' },
];

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

const toDate = (timestamp) => {
  if (typeof timestamp !== 'number' || Number.isNaN(timestamp) || !Number.isFinite(timestamp)) {
    return null;
  }
  return new Date(timestamp * 1000);
};

// Create date formatter
const dateFormatter = new Intl.DateTimeFormat(undefined, {
  year: 'numeric',
  month: 'short',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
});

// EventRow component for rendering individual rows
const EventRow = memo(function EventRow({ item: event, rowIndex, style, onContextMenu, searchQuery, caseInsensitive }) {
  const pathSource = event?.path ?? '';
  const { name, directory } = splitPath(pathSource);
  const timestamp = event?.timestamp;
  
  const date = toDate(timestamp);
  const formattedDate = date ? dateFormatter.format(date) : '—';

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
      <div className="event-time-column">
        <span className="event-time-primary">{formattedDate}</span>
      </div>
    </div>
  );
});

const FSEventsPanel = ({
  events,
  onResizeStart,
  onContextMenu,
  onHeaderContextMenu,
  searchQuery,
  caseInsensitive,
}) => {
  const headerRef = useRef(null);
  const listRef = useRef(null);

  // Handle scrolling - sync horizontal scroll to header
  const handleScroll = useCallback(({ scrollLeft }) => {
    if (headerRef.current) {
      headerRef.current.scrollLeft = scrollLeft;
    }
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

  return (
    <div className="events-panel-wrapper">
      <div ref={headerRef} className="header-row-container">
        <div className="header-row columns-events" onContextMenu={onHeaderContextMenu}>
          {COLUMNS.map(({ key, label }, index) => (
            <span key={key} className={`event-${key}-header header header-cell`}>
              {label}
              {index < COLUMNS.length - 1 && (
                <span
                  className="col-resizer"
                  onMouseDown={(e) => onResizeStart(e, key)}
                  role="separator"
                  aria-orientation="vertical"
                />
              )}
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
};

export default FSEventsPanel;
