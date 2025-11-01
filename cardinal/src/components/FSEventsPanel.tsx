import React, {
  useCallback,
  useRef,
  memo,
  useEffect,
  useImperativeHandle,
  forwardRef,
} from 'react';
import AutoSizer from 'react-virtualized/dist/commonjs/AutoSizer';
import List from 'react-virtualized/dist/commonjs/List';
import type { List as VirtualizedList, ListRowProps } from 'react-virtualized';
import 'react-virtualized/styles.css';
import { ROW_HEIGHT } from '../constants';
import { MiddleEllipsisHighlight } from './MiddleEllipsisHighlight';
import { formatTimestamp } from '../utils/format';

const COLUMNS = [
  { key: 'time', label: 'Time' },
  { key: 'name', label: 'Filename' },
  { key: 'path', label: 'Path' },
] as const;

type EventColumnKey = (typeof COLUMNS)[number]['key'];

// Distance (px) from the bottom that still counts as "user is at the end".
const BOTTOM_THRESHOLD = 50;

export type FileSystemEvent = {
  path?: string;
  timestamp?: number;
  [key: string]: unknown;
};

type EventRowProps = {
  item: FileSystemEvent | undefined;
  rowIndex: number;
  style: React.CSSProperties;
  onContextMenu?: (event: React.MouseEvent<HTMLDivElement>, path: string) => void;
  searchQuery: string;
  caseInsensitive: boolean;
};

const splitPath = (path: string | undefined): { name: string; directory: string } => {
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

const EventRow = memo(function EventRow({
  item: event,
  rowIndex,
  style,
  onContextMenu,
  searchQuery,
  caseInsensitive,
}: EventRowProps): React.JSX.Element {
  const pathSource = event?.path ?? '';
  const { name, directory } = splitPath(pathSource);
  const timestamp = typeof event?.timestamp === 'number' ? event.timestamp : undefined;
  const formattedDate = formatTimestamp(timestamp) || '—';

  const handleContextMenu = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
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

EventRow.displayName = 'EventRow';

type FSEventsPanelProps = {
  events: FileSystemEvent[];
  onResizeStart: (event: React.MouseEvent<HTMLSpanElement>, columnKey: EventColumnKey) => void;
  onContextMenu?: (event: React.MouseEvent<HTMLDivElement>, path: string) => void;
  onHeaderContextMenu?: (event: React.MouseEvent<HTMLDivElement>) => void;
  searchQuery: string;
  caseInsensitive: boolean;
};

export type FSEventsPanelHandle = {
  scrollToBottom: () => void;
};

const FSEventsPanel = forwardRef<FSEventsPanelHandle, FSEventsPanelProps>(
  ({ events, onResizeStart, onContextMenu, onHeaderContextMenu, searchQuery, caseInsensitive }, ref) => {
    const headerRef = useRef<HTMLDivElement | null>(null);
    const listRef = useRef<VirtualizedList | null>(null);
    const isAtBottomRef = useRef(true); // Track whether the viewport is watching the newest events.
    const prevEventsLengthRef = useRef(events.length);

    // Allow the parent (App) to imperatively jump to the latest event after tab switches.
    useImperativeHandle(
      ref,
      () => ({
        scrollToBottom: () => {
          const list = listRef.current;
          if (!list || events.length === 0) return;

          list.scrollToRow(events.length - 1);
          isAtBottomRef.current = true; // Mark as at bottom.
        },
      }),
      [events.length],
    );

    // Track viewport proximity to the bottom so streams only auto-scroll when the user expects it.
    const handleScroll = useCallback(
      ({ scrollTop, scrollHeight, clientHeight }: { scrollTop: number; scrollHeight: number; clientHeight: number }) => {
        const distanceFromBottom = scrollHeight - (scrollTop + clientHeight);
        isAtBottomRef.current = distanceFromBottom <= BOTTOM_THRESHOLD;
      },
      [],
    );

    // Mirror the virtualized grid's horizontal scroll onto the sticky header element.
    useEffect(() => {
      const list = listRef.current;
      const grid = list?.Grid as { _scrollingContainer?: HTMLElement } | undefined;
      const gridElement = grid?._scrollingContainer;
      if (!gridElement) return;

      const handleHorizontalScroll = () => {
        if (headerRef.current) {
          headerRef.current.scrollLeft = gridElement.scrollLeft;
        }
      };

      gridElement.addEventListener('scroll', handleHorizontalScroll);
      return () => {
        gridElement.removeEventListener('scroll', handleHorizontalScroll);
      };
    }, []);

    // Render individual row.
    const rowRenderer = useCallback(
      ({ index, key, style }: ListRowProps) => {
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

    // Keep appending events visible when the user is already watching the feed tail.
    useEffect(() => {
      const prevLength = prevEventsLengthRef.current;
      const currentLength = events.length;
      prevEventsLengthRef.current = currentLength;

      if (currentLength > prevLength && isAtBottomRef.current) {
        const list = listRef.current;
        if (list && currentLength > 0) {
          queueMicrotask(() => {
            listRef.current?.scrollToRow(currentLength - 1);
          });
        }
      }
    }, [events.length]);

    return (
      <div className="events-panel-wrapper">
        <div ref={headerRef} className="header-row-container">
          <div className="header-row columns-events" onContextMenu={onHeaderContextMenu}>
            {COLUMNS.map(({ key, label }) => (
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
              {({ width, height }: { width: number; height: number }) => (
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
  },
);

FSEventsPanel.displayName = 'FSEventsPanel';

export default FSEventsPanel;
