import React, { useEffect, useRef } from 'react';
import { createPortal } from 'react-dom';

export type ContextMenuItem = {
  label: string;
  action: () => void;
};

type ContextMenuProps = {
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
};

export function ContextMenu({ x, y, items, onClose }: ContextMenuProps): React.JSX.Element {
  const menuRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    // Close the menu whenever the user clicks anywhere outside the menu surface.
    const handleClickOutside = (event: MouseEvent) => {
      const target = event.target as Node | null;
      if (menuRef.current && target && !menuRef.current.contains(target)) {
        onClose();
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [onClose]);

  const handleItemClick = (action: () => void) => {
    // Invoke the menu action first, then tear down the menu overlay.
    action();
    onClose();
  };

  return createPortal(
    <div ref={menuRef} className="context-menu" style={{ top: y, left: x }}>
      <ul>
        {items.map((item) => (
          <li key={item.label} onClick={() => handleItemClick(item.action)}>
            {item.label}
          </li>
        ))}
      </ul>
    </div>,
    document.body,
  );
}
