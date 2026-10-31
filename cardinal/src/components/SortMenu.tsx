import { useCallback, useEffect, useRef, useState } from 'react';
import type { SortDirection, SortKey } from '../types/sort';

export type SortMenuSelection = { key: SortKey; direction: SortDirection } | null;

type SortMenuProps = {
  /** Sort currently applied to this tab, so the menu can show a check mark. */
  sortState: { key: SortKey; direction: SortDirection } | null;
  onSelect: (selection: SortMenuSelection) => void;
  labels: {
    sortBySize: string;
    sortByCreated: string;
    sortByModified: string;
    sortByName: string;
    ascending: string;
    descending: string;
    clear: string;
  };
  /** Viewport coordinates where the menu should open. */
  x: number;
  y: number;
  onClose: () => void;
};

type SortOption = { key: SortKey; label: string };

const MENU_WIDTH = 200;
const SUBMENU_WIDTH = 130;

/**
 * Right-click sort menu with a hover submenu per key (ascending / descending).
 * Implemented in HTML rather than the native menu because the native menu API has no
 * submenu support, and the sort must apply only to the tab that opened it.
 */
export function SortMenu({
  sortState,
  onSelect,
  labels,
  x,
  y,
  onClose,
}: SortMenuProps): React.JSX.Element {
  const [openSubmenu, setOpenSubmenu] = useState<SortKey | null>(null);
  const [position, setPosition] = useState({ x, y });
  const menuRef = useRef<HTMLDivElement | null>(null);
  const hoverTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const options: SortOption[] = [
    { key: 'size', label: labels.sortBySize },
    { key: 'ctime', label: labels.sortByCreated },
    { key: 'mtime', label: labels.sortByModified },
    { key: 'filename', label: labels.sortByName },
  ];

  // Keep the whole menu (including the submenu) inside the window.
  useEffect(() => {
    const menu = menuRef.current;
    const height = menu?.offsetHeight ?? 0;
    const maxX = window.innerWidth - MENU_WIDTH - SUBMENU_WIDTH - 8;
    const maxY = window.innerHeight - height - 8;
    setPosition({ x: Math.max(8, Math.min(x, maxX)), y: Math.max(8, Math.min(y, maxY)) });
  }, [x, y]);

  useEffect(() => {
    const handlePointerDown = (event: PointerEvent) => {
      if (!menuRef.current?.contains(event.target as Node)) {
        onClose();
      }
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose();
      }
    };
    window.addEventListener('pointerdown', handlePointerDown, true);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('pointerdown', handlePointerDown, true);
      window.removeEventListener('keydown', handleKeyDown);
      if (hoverTimerRef.current) {
        clearTimeout(hoverTimerRef.current);
      }
    };
  }, [onClose]);

  const choose = useCallback(
    (key: SortKey, direction: SortDirection) => {
      onSelect({ key, direction });
      onClose();
    },
    [onClose, onSelect],
  );

  const isActive = (key: SortKey, direction: SortDirection) =>
    sortState?.key === key && sortState.direction === direction;

  return (
    <div
      ref={menuRef}
      className="sort-menu"
      style={{ left: position.x, top: position.y }}
      role="menu"
      onContextMenu={(event) => event.preventDefault()}
    >
      {options.map((option) => {
        const isOpen = openSubmenu === option.key;
        return (
          <div
            key={option.key}
            className={`sort-menu__item${isOpen ? ' is-open' : ''}`}
            role="menuitem"
            tabIndex={0}
            aria-haspopup="menu"
            aria-expanded={isOpen}
            onPointerEnter={() => setOpenSubmenu(option.key)}
            onFocus={() => setOpenSubmenu(option.key)}
            onKeyDown={(event) => {
              if (event.key === 'ArrowRight' || event.key === 'Enter') {
                setOpenSubmenu(option.key);
              }
            }}
          >
            <span className="sort-menu__label">{option.label}</span>
            <span className="sort-menu__chevron" aria-hidden="true">
              ›
            </span>

            {isOpen ? (
              <div className="sort-submenu" role="menu">
                {(['asc', 'desc'] as const).map((direction) => (
                  <button
                    key={direction}
                    type="button"
                    role="menuitemradio"
                    aria-checked={isActive(option.key, direction)}
                    className={`sort-submenu__item${
                      isActive(option.key, direction) ? ' is-active' : ''
                    }`}
                    onClick={() => choose(option.key, direction)}
                  >
                    {direction === 'asc' ? labels.ascending : labels.descending}
                  </button>
                ))}
              </div>
            ) : null}
          </div>
        );
      })}

      {sortState ? (
        <button
          type="button"
          role="menuitem"
          className="sort-menu__item sort-menu__item--clear"
          onClick={() => {
            onSelect(null);
            onClose();
          }}
        >
          {labels.clear}
        </button>
      ) : null}
    </div>
  );
}

export default SortMenu;
